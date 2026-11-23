pub mod expressions;
pub mod statements;
pub mod types;

use expressions::convert_expr;
use statements::convert_block_stmt;
use types::{WithExtraTypes, convert_named_type, convert_type_element, convert_unnamed_type};

use crate::{
    ast::{
        ConstSpec, Decl, Declaration, File, FuncDecl, FuncType, Import, ParameterSpec, Type,
        TypeSpec, VarSpec,
    },
    name_resolution::Context,
};

pub fn convert_ast(file: File, context: &mut Context) -> Result<String, String> {
    let declarations = file
        .decl
        .iter()
        .map(|decl| convert_decl(decl, context))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(declarations.join("\n"))
}

fn convert_import(import: &Import, _context: &mut Context) -> Result<String, String> {
    let name = (import.name.as_ref().map(|i| i.name.as_str()))
        .or_else(|| import.path.value.split('/').last());
    if let Some(name) = name {
        return Ok(format!("use {name}::*;"));
    } else {
        return Err(format!("Not a valid package {}", import.path.value));
    }
}

fn convert_decl(decl: &Declaration, context: &mut Context) -> Result<String, String> {
    match decl {
        Declaration::Type(type_decl) => Ok(convert_type_decl(type_decl, context)),
        Declaration::Function(func_decl) => Ok(convert_function(func_decl, context)),
        Declaration::Const(const_decl) => Ok(convert_const_decl(const_decl, context)),
        Declaration::Variable(var_decl) => Ok(convert_variable_decl(var_decl, context)),
    }
}

fn convert_type_decl(type_d: &Decl<TypeSpec>, context: &mut Context) -> String {
    let mut types = String::new();
    for spec in &type_d.specs {
        context
            .types
            .insert(spec.name.name.clone(), spec.typ.clone());
        types += &convert_named_type(&spec.name.name, &spec.typ, context).typ;
    }
    types
}

pub fn convert_const_decl(const_decl: &Decl<ConstSpec>, context: &mut Context) -> String {
    let mut extras = vec![];
    let mut declarations = vec![];
    for spec in &const_decl.specs {
        let typ = if let Some(typ) = &spec.typ {
            let WithExtraTypes {
                typ,
                extras: const_extras,
            } = convert_unnamed_type(&typ, context);
            extras.extend(const_extras);
            format!(": {typ}")
        } else {
            ": _".to_string()
        };
        for (name, value) in spec.name.iter().zip(spec.values.iter()) {
            let name = &name.name;
            let value = convert_expr(value, context);
            declarations.push(format!("const {name}{typ} = {value};"))
        }
    }

    declarations.join("\n")
}

/// Converts variable declarations to rust syntax.
/// This currently only outputs variables declared as static mut such as:
/// ```
/// static mut variable=value;
/// ```
/// This means that global variables are not yet properly supported and all variables are treated as mutable.
/// This function does not perform any type inference so will only output types on variables if they are present in the original Go source.
/// Multi variable declarations are split into multiple single variable declarations in rust.
///
/// Note This does not yet output declarations for anonymous types.
pub fn convert_variable_decl(var_decl: &Decl<VarSpec>, context: &mut Context) -> String {
    let mut extras = vec![];
    let mut declarations = vec![];
    for spec in &var_decl.specs {
        let typ = if let Some(typ) = &spec.typ {
            let WithExtraTypes {
                typ,
                extras: var_extras,
            } = convert_unnamed_type(&typ, context);
            extras.extend(var_extras);
            format!(": {typ}")
        } else {
            ": _".to_string()
        };
        for (name, value) in spec.name.iter().zip(spec.values.iter()) {
            let name = &name.name;
            let value = convert_expr(value, context);
            declarations.push(format!("static mut {name}{typ} = {value};"))
        }
    }

    declarations.join("\n")
}

fn convert_function(func: &FuncDecl, context: &mut Context) -> String {
    let signature =
        convert_function_signature(func.name.name.as_str(), &func.recv, &func.typ, context);
    let recv = &func.recv;

    let func = if let Some(body) = &func.body {
        let statements = convert_block_stmt(body, context);
        // Note that `convert_block_stmt` wraps the block in braces so we don't need to wrap the body of the function in braces here
        format!("{signature}{statements}")
    } else {
        format!("unsafe extern \"C\" {{{signature};}}")
    };
    if let Some(recv) = recv {
        let recv_type = &recv[0].typ;
        let recv_type = match recv_type {
            Type::Name(_) => recv_type,
            Type::Pointer(typ) => &*typ.typ,
            _ => unreachable!(
                "Reciever types can only be a defined type or pointer to a defined type"
            ),
        };
        let recv_type = convert_unnamed_type(recv_type, context).typ;
        format!("impl {recv_type} {{\n{func}\n}}")
    } else {
        func
    }
}

pub fn convert_function_signature(
    name: &str,
    recv: &Option<Vec<ParameterSpec>>,
    func: &FuncType,
    context: &mut Context,
) -> String {
    let mut type_params = vec![];
    let mut extras = vec![];
    for arg in &func.typ_params {
        let WithExtraTypes {
            typ,
            extras: arg_extras,
        } = convert_type_element(&arg.constraint, context);
        extras.extend(arg_extras);
        for name in &arg.name {
            let name = &name.name;
            type_params.push(format!("{name}: {typ}"));
        }
    }

    let type_params = if !type_params.is_empty() {
        let types = type_params.join(", ");
        format!("<{types}>")
    } else {
        String::new()
    };

    let mut params = vec![];
    for param in &func.params {
        let WithExtraTypes {
            typ,
            extras: param_extras,
        } = convert_unnamed_type(&param.typ, context);
        extras.extend(param_extras);

        match param.identifiers.as_slice() {
            [] => {
                // We convert variadic parameters into vectors for now
                if param.variadic {
                    params.push(format!("_: Vec<{typ}>"));
                } else {
                    params.push(format!("_: {typ}"));
                }
            }
            [name] => {
                let name = &name.name;
                // We convert variadic parameters into vectors for now
                if param.variadic {
                    params.push(format!("{name}: Vec<{typ}>"));
                } else {
                    params.push(format!("{name}: {typ}"));
                }
            }
            names => {
                for (index, name) in names.iter().enumerate() {
                    let name = &name.name;
                    // We convert variadic parameters into vectors for now
                    if param.variadic && index == names.len() - 1 {
                        params.push(format!("{name}: Vec<{typ}>"));
                    } else {
                        params.push(format!("{name}: {typ}"));
                    }
                }
            }
        };
    }
    if let Some(recv) = recv {
        let name = &recv[0].identifiers.first();
        match name {
            None => params.insert(0, "&self".to_string()),
            Some(name) => params.insert(0, format!("{ }:&mut Self", &name.name)),
        }
    }
    let params = params.join(", ");

    let mut result = vec![];

    for res in &func.result {
        let WithExtraTypes {
            typ,
            extras: res_extras,
        } = convert_unnamed_type(&res.typ, context);
        extras.extend(res_extras);
        // unnamed
        if res.identifiers.is_empty() {
            result.push(typ.clone());
        } else {
            for _name in &res.identifiers {
                result.push(typ.clone());
            }
        }
    }

    let result = match result.as_slice() {
        [] => String::new(),
        [res] => format!("->{res}"),
        res => {
            let res = res.join(", ");
            format!("->({res})")
        }
    };

    format!("fn {name}{type_params}({params}){result}")
}

#[cfg(test)]
mod tests {
    use gosyn::Parser;

    use super::convert_ast;
    use crate::Context;

    /// Transforms the golang string into rust then asserts that it is exactly equal to the given rust string.
    fn assert_transformation(golang: &str, rust: &str) {
        let mut parser = Parser::from(golang);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file.into(), &mut Context::default()).unwrap();
        assert_eq!(res, rust);
    }

    #[test]
    fn transform_empty_struct_type() {
        assert_transformation("package main;\ntype test struct {}", "struct test{\n\n}");
    }

    #[test]
    fn transform_struct_type_with_named_fields() {
        assert_transformation(
            "package main;\ntype test struct {first i64; second string}",
            "struct test{\nfirst: i64,\nsecond: string,\n}",
        );
    }

    #[test]
    fn transform_struct_type_with_named_fields_with_generic() {
        assert_transformation(
            "package main;\ntype test struct {first i64; second t[string]}",
            "struct test{\nfirst: i64,\nsecond: t<string>,\n}",
        );
    }

    #[test]
    fn transform_struct_type_with_named_fields_with_multi_generics() {
        assert_transformation(
            "package main;\ntype test struct {first i64; second t[string,i32, fourth[fifth,sixth]]}",
            "struct test{\nfirst: i64,\nsecond: t<string, i32, fourth<fifth, sixth>>,\n}",
        );
    }

    #[test]
    fn transform_struct_type_with_unnamed_fields() {
        assert_transformation(
            "package main;\ntype test struct {i64; string}",
            "struct test{\ni64: i64,\nstring: string,\n}",
        );
    }

    #[test]
    fn transform_struct_type_with_qualified_unnamed_fields() {
        assert_transformation(
            "package main;\ntype test struct {i64; t.second}",
            "struct test{\ni64: i64,\nsecond: t::second,\n}",
        );
    }

    #[test]
    fn transform_struct_type_with_unnamed_fields_with_generics() {
        assert_transformation(
            "package main;\ntype test struct {i64; t[third]}",
            "struct test{\ni64: i64,\nt: t<third>,\n}",
        );
    }

    #[test]
    fn transform_struct_type_with_unnamed_fields_with_multi_generics() {
        assert_transformation(
            "package main;\ntype test struct {i64; t[third, fourth, fifth.sixth]}",
            "struct test{\ni64: i64,\nt: t<third, fourth, fifth::sixth>,\n}",
        );
    }

    #[test]
    fn transform_struct_type_with_qualified_unnamed_fields_with_generics() {
        assert_transformation(
            "package main;\ntype test struct {i64; t.second[third]}",
            "struct test{\ni64: i64,\nsecond: t::second<third>,\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_no_params_or_result() {
        assert_transformation("package main;\nfunc test () {}", "fn test(){\n\n}");
    }

    #[test]
    fn transform_func_decl_with_single_named_params_and_no_result() {
        assert_transformation(
            "package main;\nfunc test (count i64) {}",
            "fn test(count: i64){\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_multi_name_single_type_params_and_no_result() {
        assert_transformation(
            "package main;\nfunc test (a, b i64) {}",
            "fn test(a: i64, b: i64){\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_multi_name_multi_type_params_and_no_result() {
        assert_transformation(
            "package main;\nfunc test (first string, second i64) {}",
            "fn test(first: string, second: i64){\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_no_params_and_single_unnamed_result() {
        assert_transformation(
            "package main;\nfunc test ()string {}",
            "fn test()->string{\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_no_params_and_multi_unnamed_result() {
        assert_transformation(
            "package main;\nfunc test ()(string,error) {}",
            "fn test()->(string, error){\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_no_params_and_single_named_result() {
        assert_transformation(
            "package main;\nfunc test ()(name string) {}",
            "fn test()->string{\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_no_params_and_multi_named_single_type_result() {
        assert_transformation(
            "package main;\nfunc test ()(first, last string) {}",
            "fn test()->(string, string){\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_no_params_or_result_or_body() {
        assert_transformation(
            "package main;\nfunc test ()",
            "unsafe extern \"C\" {fn test();}",
        );
    }

    #[test]
    fn transform_func_decl_with_params_and_result_with_simple_binary_expression() {
        assert_transformation(
            "package main;\nfunc test (a, b int)int{a+b}",
            "fn test(a: int, b: int)->int{\na + b\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_and_result_with_simple_binary_expression_on_int_literals() {
        assert_transformation(
            "package main;\nfunc test ()int{1+2}",
            "fn test()->int{\n1 + 2\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_and_result_with_simple_binary_expression_on_string_literals() {
        assert_transformation(
            "package main;\nfunc test ()int{\"hello\" + \"world\"}",
            "fn test()->int{\n\"hello\" + \"world\"\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_result_with_binary_expression_with_parens() {
        assert_transformation(
            "package main;\nfunc test ()int{1+(2+3)}",
            "fn test()->int{\n1 + (2 + 3)\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_slice_param() {
        assert_transformation(
            "package main;\nfunc test (arg []first){\n\n}",
            "fn test(arg: &[first]){\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_array_param() {
        assert_transformation(
            "package main;\nfunc test (arg [5]first){\n\n}",
            "fn test(arg: [first; 5]){\n\n}",
        );
    }

    #[test]
    fn transform_func_decl_with_map_param() {
        assert_transformation(
            "package main;\nfunc test (arg map[int]string){\n\n}",
            "fn test(arg: HashMap<int, string>){\n\n}",
        );
    }

    #[test]
    fn transform_block_statements() {
        assert_transformation(
            "package main;\nfunc test (){a+b;{c+d;}e+f}",
            "fn test(){\na + b;\n{\nc + d\n};\ne + f\n}",
        );
    }

    #[test]
    fn transform_func_with_return_statement_single() {
        assert_transformation(
            "package main;\nfunc test ()int{a+b;return c+d}",
            "fn test()->int{\na + b;\nreturn c + d;\n}",
        );
    }

    #[test]
    fn transform_func_with_return_statement_multi() {
        assert_transformation(
            "package main;\nfunc test ()(int, string){a+b;return c+d, \"hello\"}",
            "fn test()->(int, string){\na + b;\nreturn (c + d, \"hello\");\n}",
        );
    }

    #[test]
    fn transform_func_with_return_statement_named_multi() {
        assert_transformation(
            "package main;\nfunc test ()(count int, err error){a+b;return count, error}",
            "fn test()->(int, error){\na + b;\nreturn (count, error);\n}",
        );
    }

    #[test]
    fn transform_func_with_simple_assign_statement() {
        assert_transformation("package main;\nfunc test (){a=b}", "fn test(){\na = b\n}");
    }

    #[test]
    fn transform_func_with_multi_assign_statement() {
        assert_transformation(
            "package main;\nfunc test (){a, b=c, d}",
            "fn test(){\na = c;\nb = d\n}",
        );
    }

    #[test]
    fn transform_func_with_plus_assign_statement() {
        assert_transformation("package main;\nfunc test (){a+=b}", "fn test(){\na += b\n}");
    }

    #[test]
    fn transform_func_with_simple_call_expression() {
        assert_transformation(
            "package main;\nfunc test (){myfunc()}",
            "fn test(){\nmyfunc()\n}",
        );
    }

    #[test]
    fn transform_func_with_call_expression_with_arguments() {
        assert_transformation(
            "package main;\nfunc test (){myfunc(a,b)}",
            "fn test(){\nmyfunc(a, b)\n}",
        );
    }

    #[test]
    fn transform_short_variable_declaration() {
        assert_transformation(
            "package main;\nfunc test (){a:=5}",
            "fn test(){\nlet mut a = 5;\n}",
        );
    }

    #[test]
    fn transform_short_variable_declaration_with_multi_name() {
        assert_transformation(
            "package main;\nfunc test (){ok, err:=calculate()}",
            "fn test(){\nlet (mut ok, mut err) = calculate();\n}",
        );
    }

    #[test]
    fn transform_short_variable_declaration_with_multi_name_and_expression() {
        assert_transformation(
            "package main;\nfunc test (){ok, err:=2, 5}",
            "fn test(){\nlet (mut ok, mut err) = (2, 5);\n}",
        );
    }

    #[test]
    fn transform_simple_if_statement() {
        assert_transformation(
            "package main;\nfunc test (){if a>b{do_thing()}}",
            "fn test(){\nif a > b {\ndo_thing()\n}\n}",
        );
    }

    #[test]
    fn transform_if_statement_with_else() {
        assert_transformation(
            "package main;\nfunc test (){if a>b{do_thing()}else {do_other_thing()};}",
            "fn test(){\nif a > b {\ndo_thing()\n} else {\ndo_other_thing()\n}\n}",
        );
    }

    #[test]
    fn transform_if_statement_with_if_else() {
        assert_transformation(
            "package main;\nfunc test (){if a>b{do_thing()}else if a<b {do_other_thing()} else{do_another_thing()};}",
            "fn test(){\nif a > b {\ndo_thing()\n} else if a < b {\ndo_other_thing()\n} else {\ndo_another_thing()\n}\n}",
        );
    }

    #[test]
    fn transform_if_statement_with_else_and_init() {
        assert_transformation(
            "package main;\nfunc test (){if a,b:=calc(); a>b{do_thing()}else {do_other_thing()};}",
            "fn test(){\n{\nlet (mut a, mut b) = calc();\nif a > b {\ndo_thing()\n} else {\ndo_other_thing()\n}\n}\n}",
        );
    }

    #[test]
    fn transform_function_literal() {
        assert_transformation(
            "package main;\nfunc test (){a=func(a, b i64){a+b}}",
            "fn test(){\na = |a: i64, b: i64|{\na + b\n}\n}",
        );
    }

    #[test]
    fn transform_simple_selector() {
        assert_transformation(
            "package main;\nfunc test (box square){box.side_length}",
            "fn test(box: square){\nbox.side_length\n}",
        );
    }

    #[test]
    fn transform_selector_after_call() {
        assert_transformation(
            "package main;\nfunc test (){get_box().side_length}",
            "fn test(){\nget_box().side_length\n}",
        );
    }

    #[test]
    fn transform_call_from_selector() {
        assert_transformation(
            "package main;\nfunc test (box square){box.side_length()}",
            "fn test(box: square){\nbox.side_length()\n}",
        );
    }

    #[test]
    fn transform_simple_index() {
        assert_transformation(
            "package main;\nfunc test (arr []things){arr[0]}",
            "fn test(arr: &[things]){\narr[0]\n}",
        );
    }

    #[test]
    fn transform_index_after_call() {
        assert_transformation(
            "package main;\nfunc test (){get_array()[index]}",
            "fn test(){\nget_array()[index]\n}",
        );
    }

    #[test]
    fn transform_call_from_index() {
        assert_transformation(
            "package main;\nfunc test (funcs []func_t){funcs[0]()}",
            "fn test(funcs: &[func_t]){\nfuncs[0]()\n}",
        );
    }

    #[test]
    fn transform_empty_for_loop() {
        assert_transformation(
            "package main;\nfunc test (){for{}}",
            "fn test(){\nloop {\n\n}\n}",
        );
    }

    #[test]
    fn transform_for_loop_with_cond() {
        assert_transformation(
            "package main;\nfunc test (){for i<10 {}}",
            "fn test(){\nwhile i < 10{\n\n}\n}",
        );
    }

    #[test]
    fn transform_for_loop_with_init_and_cond() {
        assert_transformation(
            "package main;\nfunc test (){for i:=0; i<10; {}}",
            "fn test(){\n{\nlet mut i = 0;\nwhile i < 10{\n\n}\n}\n}",
        );
    }

    #[test]
    fn transform_for_loop_with_init_and_cond_and_post() {
        assert_transformation(
            "package main;\nfunc test (){for i:=0; i<10;i+=1 {}}",
            "fn test(){\n{\nlet mut i = 0;\nwhile i < 10{\n{\n\n}\ni += 1\n}\n}\n}",
        );
    }

    #[test]
    fn transform_empty_struct_literal() {
        assert_transformation(
            "package main;\nfunc test (){Point{}}",
            "fn test(){\nPoint {\n\n}\n}",
        );
    }

    #[test]
    fn transform_struct_literal_with_named_fields() {
        assert_transformation(
            "package main;\nfunc test (){Point{x:4,y:2}}",
            "fn test(){\nPoint {\nx: 4,\ny: 2,\n}\n}",
        );
    }

    #[test]
    fn transform_struct_literal_with_unnamed_fields() {
        assert_transformation(
            "package main;\nfunc test (){Point{4,2}}",
            "fn test(){\nPoint::new(4, 2)\n}",
        );
    }

    #[test]
    fn transform_map_literal_empty() {
        assert_transformation(
            "package main;\nfunc test (){map[string]int{}}",
            "fn test(){\n{\nlet mut map = HashMap::<string, int>::new();\n\nmap\n}\n}",
        );
    }

    #[test]
    fn transform_map_literal_with_elements() {
        assert_transformation(
            "package main;\nfunc test (){map[string]int{\"first\":1,\"second\":2}}",
            "fn test(){\n{\nlet mut map = HashMap::<string, int>::new();\nmap.insert(\"first\", 1);\nmap.insert(\"second\", 2);\nmap\n}\n}",
        );
    }

    #[test]
    fn transform_array_literal_empty() {
        assert_transformation(
            "package main;\nfunc test (){[0]int{}}",
            "fn test(){\n{\nlet array: [int; 0] = [];\narray\n}\n}",
        );
    }

    #[test]
    fn transform_array_literal_with_elements() {
        assert_transformation(
            "package main;\nfunc test (){[2]int{4,5}}",
            "fn test(){\n{\nlet array: [int; 2] = [4, 5];\narray\n}\n}",
        );
    }

    #[test]
    fn transform_slice_literal_empty() {
        assert_transformation(
            "package main;\nfunc test (){[]int{}}",
            "fn test(){\n{\nlet array: [int; 0] = [];\narray\n}.as_slice()\n}",
        );
    }

    #[test]
    fn transform_slice_literal_with_elements() {
        assert_transformation(
            "package main;\nfunc test (){[]int{4,5}}",
            "fn test(){\n{\nlet array: [int; 2] = [4, 5];\narray\n}.as_slice()\n}",
        );
    }

    #[test]
    fn transform_unary_sub() {
        assert_transformation("package main;\nfunc test (){-4}", "fn test(){\n-4\n}");
    }

    #[test]
    fn transform_reference() {
        assert_transformation(
            "package main;\nfunc test (){y:=&x}",
            "fn test(){\nlet mut y = &x;\n}",
        );
    }

    #[test]
    fn transform_pointer_dereference() {
        assert_transformation(
            "package main;\nfunc test (){y:=*x}",
            "fn test(){\nlet mut y = *x;\n}",
        );
    }

    #[test]
    fn transform_method_declaration_with_unnamed_reciever() {
        assert_transformation(
            "package main;\nfunc(*Box) test (){}",
            "impl Box {\nfn test(&self){\n\n}\n}",
        );
    }

    #[test]
    fn transform_method_declaration_with_named_reciever() {
        assert_transformation(
            "package main;\nfunc(b *Box) test (){}",
            "impl Box {\nfn test(b:&mut Self){\n\n}\n}",
        );
    }

    #[test]
    fn transform_pointer_type() {
        assert_transformation(
            "package main;\nfunc test (b *Box){}",
            "fn test(b: &mut Box){\n\n}",
        );
    }

    #[test]
    fn transform_paren_type() {
        assert_transformation(
            "package main;\nfunc test (b (Box)){}",
            "fn test(b: (Box)){\n\n}",
        );
    }

    #[test]
    fn transform_func_type_empty() {
        assert_transformation(
            "package main;\nfunc test (f func()){}",
            "fn test(f: Fn()){\n\n}",
        );
    }

    #[test]
    fn transform_func_type_with_unnamed_params() {
        assert_transformation(
            "package main;\nfunc test (f func(i64,string)){}",
            "fn test(f: Fn(i64, string)){\n\n}",
        );
    }

    #[test]
    fn transform_func_type_with_named_params() {
        assert_transformation(
            "package main;\nfunc test (f func(first, second string)){}",
            "fn test(f: Fn(string, string)){\n\n}",
        );
    }

    #[test]
    fn transform_func_type_with_return_type() {
        assert_transformation(
            "package main;\nfunc test (f func()string){}",
            "fn test(f: Fn()->string){\n\n}",
        );
    }

    #[test]
    fn transform_simple_range_statement() {
        assert_transformation(
            "package main;\nfunc test (){ for i := range array {} }",
            "fn test(){\nfor i in array {\n\n}\n}",
        );
    }

    #[test]
    fn transform_range_statement_with_key_and_val() {
        assert_transformation(
            "package main;\nfunc test (){ for k, v := range hashmap {} }",
            "fn test(){\nfor (k, v) in hashmap {\n\n}\n}",
        );
    }

    #[test]
    fn transform_const_single_with_type() {
        assert_transformation(
            "package main;\nconst PI float64=3.1415",
            "const PI: float64 = 3.1415;",
        );
    }

    #[test]
    fn transform_const_two_with_type() {
        assert_transformation(
            "package main;\nconst ( PI float64=3.1415\ntau float64 = 6.2832 )",
            "const PI: float64 = 3.1415;\nconst tau: float64 = 6.2832;",
        );
    }

    #[test]
    fn transform_const_two_without_one_type() {
        assert_transformation(
            "package main;\nconst ( PI float64=3.1415\ntau = 6.2832 )",
            "const PI: float64 = 3.1415;\nconst tau: _ = 6.2832;",
        );
    }

    #[test]
    fn transform_break() {
        assert_transformation(
            "package main;\nfunc test(){for {break}}",
            "fn test(){\nloop {\nbreak;\n}\n}",
        );
    }

    #[test]
    fn transform_break_with_label() {
        assert_transformation(
            "package main;\nfunc test(){for {break outter}}",
            "fn test(){\nloop {\nbreak 'outter;\n}\n}",
        );
    }

    #[test]
    fn transform_continue() {
        assert_transformation(
            "package main;\nfunc test(){for {continue}}",
            "fn test(){\nloop {\ncontinue;\n}\n}",
        );
    }

    #[test]
    fn transform_continue_with_label() {
        assert_transformation(
            "package main;\nfunc test(){for {continue outter}}",
            "fn test(){\nloop {\ncontinue 'outter;\n}\n}",
        );
    }

    #[test]
    fn transform_function_with_type_param() {
        assert_transformation(
            "package main;\nfunc test[t any](){}",
            "fn test<t: any>(){\n\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_no_tag_and_complex_cases_and_no_default() {
        assert_transformation(
            "package main;\nfunc test(){switch{case x==0:return 0;\ncase x==1: dothing()}}",
            "fn test(){\nif (x == 0) == true {\nreturn 0;\n} else if (x == 1) == true {\ndothing()\n}\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_no_tag_and_complex_cases_and_default() {
        assert_transformation(
            "package main;\nfunc test(){switch{case x==0:return 0;\ncase x==1: dothing();default: dootherthing()}}",
            "fn test(){\nif (x == 0) == true {\nreturn 0;\n} else if (x == 1) == true {\ndothing()\n} else  {\ndootherthing()\n}\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_tag_and_complex_cases_and_no_default() {
        assert_transformation(
            "package main;\nfunc test(){switch false {case x==0:return 0;\ncase x==1: dothing()}}",
            "fn test(){\nif (x == 0) == false {\nreturn 0;\n} else if (x == 1) == false {\ndothing()\n}\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_tag_and_complex_cases_and_default() {
        assert_transformation(
            "package main;\nfunc test(){switch false{case x==0:return 0;\ncase x==1: dothing();default: dootherthing()}}",
            "fn test(){\nif (x == 0) == false {\nreturn 0;\n} else if (x == 1) == false {\ndothing()\n} else  {\ndootherthing()\n}\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_no_tag_and_complex_multi_cases_and_no_default() {
        assert_transformation(
            "package main;\nfunc test(){switch{case x==0, x==2:return 0;\ncase x==1: dothing()}}",
            "fn test(){\nif (x == 0) == true || (x == 2) == true {\nreturn 0;\n} else if (x == 1) == true {\ndothing()\n}\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_tag_and_simple_cases_and_no_default() {
        assert_transformation(
            "package main;\nfunc test(){switch number {case 0:return 0;\ncase one: dothing()}}",
            "fn test(){\nmatch number {\n0 => {\nreturn 0;\n}\none => {\ndothing()\n}\n}\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_tag_and_simple_cases_and_default() {
        assert_transformation(
            "package main;\nfunc test(){switch number {case 0:return 0;\ncase one: dothing();default: dootherthing()}}",
            "fn test(){\nmatch number {\n0 => {\nreturn 0;\n}\none => {\ndothing()\n}\n_ => {\ndootherthing()\n}\n}\n}",
        );
    }

    #[test]
    fn transform_switch_stmt_with_tag_and_simple_multi_cases_and_no_default() {
        assert_transformation(
            "package main;\nfunc test(){switch number {case 0, 2:return 0;\ncase one, three: dothing()}}",
            "fn test(){\nmatch number {\n0 | 2 => {\nreturn 0;\n}\none | three => {\ndothing()\n}\n}\n}",
        );
    }

    #[test]
    fn transform_slice_with_no_indices() {
        assert_transformation("package main;\nfunc test(){x[:]}", "fn test(){\nx[..]\n}");
    }

    #[test]
    fn transform_slice_with_both_indices() {
        assert_transformation(
            "package main;\nfunc test(){x[1:4]}",
            "fn test(){\nx[1..4]\n}",
        );
    }

    #[test]
    fn transform_slice_with_first_index() {
        assert_transformation("package main;\nfunc test(){x[1:]}", "fn test(){\nx[1..]\n}");
    }

    #[test]
    fn transform_slice_with_second_index() {
        assert_transformation("package main;\nfunc test(){x[:4]}", "fn test(){\nx[..4]\n}");
    }

    #[test]
    fn transform_declaration_type_pointer() {
        assert_transformation("package main;\ntype test *int", "type test<'a> = &'a int;");
    }

    #[test]
    fn transform_declaration_type_slice() {
        assert_transformation(
            "package main;\ntype test []int",
            "type test<'a> = &'a [int];",
        );
    }

    #[test]
    fn transform_interface_with_no_elements() {
        assert_transformation("package main;\ntype test interface {}", "trait test {\n\n}");
    }

    #[test]
    fn transform_interface_with_one_method() {
        assert_transformation(
            "package main;\ntype test interface {callme() string}",
            "trait test {\nfn callme()->string;\n}",
        );
    }

    #[test]
    fn transform_type_chan() {
        assert_transformation(
            "package main;\nfunc test () chan thing{}",
            "fn test()->chan<thing>{\n\n}",
        );
    }

    #[test]
    fn transform_type_decl_for_chan() {
        assert_transformation(
            "package main;\ntype myChan=chan thing",
            "type myChan = chan<thing>;",
        );
    }

    #[test]
    fn transform_type_decl_for_array() {
        assert_transformation("package main;\ntype arr=[3]int", "type arr = [int; 3];");
    }
}
