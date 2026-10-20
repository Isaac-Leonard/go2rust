pub mod name_resolution;

use std::sync::atomic::{AtomicUsize, Ordering};

use gosyn::ast::{
    ArrayType, BlockStmt, ConstSpec, Decl, DeclStmt, Declaration, Expression, File, FuncLit,
    IfStmt, Import, InstantiatedType, MapType, Selector, ShortVarDecl, SliceType, Statement,
    StructType, Type, TypeSpec, VarSpec,
};
use gosyn::ast::{FuncDecl, InterfaceType};
use name_resolution::Context;

fn main() {
    let mut args = std::env::args();
    let Some(file_path) = args.nth(1) else {
        eprintln!("No file specified");
        return;
    };
    let ast = match gosyn::parse_file(file_path) {
        Ok(ast) => ast,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };

    println!("{:?}", ast);
}

pub fn convert_ast(file: File, context: &mut Context) -> Result<String, String> {
    let imports = file
        .imports
        .iter()
        .map(|import| convert_import(import, context))
        .try_fold(String::new(), |a, b| -> Result<String, String> {
            Ok(a + &b?)
        })?;

    let declarations = file
        .decl
        .iter()
        .map(|decl| convert_decl(decl, context))
        .try_fold(String::new(), |a, b| -> Result<String, String> {
            Ok(a + &b?)
        })?;

    return Ok(imports + &declarations);
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

pub fn convert_type_decl(type_d: &Decl<TypeSpec>, context: &mut Context) -> String {
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
    todo!()
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
            } = convert_unnamed_type(typ, context);
            extras.extend(var_extras);
            format!(": {typ}")
        } else {
            String::new()
        };
        for (name, value) in spec.name.iter().zip(spec.values.iter()) {
            let name = &name.name;
            let value = convert_expr(value, context);
            declarations.push(format!("static mut {name}{typ} = {value};"))
        }
    }

    declarations.join("\n")
}

/// Converts variable short form declarations to rust syntax.
/// This currently only outputs variables declared as let mut such as:
/// ```
/// let mut variable=value;
/// ```
/// This means that all variables are treated as mutable.
/// This function does not perform any type inference so will not output types on variables.
/// Multi variable declarations are split into multiple single variable declarations in rust.
///
pub fn convert_short_variable_decl(decl: &ShortVarDecl, context: &mut Context) -> String {
    let names = decl
        .name
        .iter()
        .map(|name| name.name.clone())
        .collect::<Vec<_>>();
    let values = decl
        .values
        .iter()
        .map(|value| convert_expr(value, context))
        .collect::<Vec<_>>();

    let names = match names.as_slice() {
        [name] => name.clone(),
        names => {
            let names = names.join(", ");
            format!("({names})")
        }
    };

    let values = match values.as_slice() {
        [value] => value.to_string(),
        values => {
            let values = values.join(", ");
            format!("({values})")
        }
    };

    format!("let mut {names} = {values};")
}

pub struct WithExtraTypes {
    typ: String,
    extras: Vec<String>,
}

fn convert_named_type(name: &str, typ: &gosyn::ast::Type, context: &mut Context) -> WithExtraTypes {
    use gosyn::ast::Type;
    match typ {
        Type::Name(t_name) => WithExtraTypes {
            typ: format!("type {} = {}", name, convert_name(t_name)),
            extras: vec![],
        },
        Type::Instantiated(InstantiatedType {
            name: t_name,
            arguments,
            ..
        }) => {
            let mut extras = vec![];
            let typ = convert_name(t_name);
            let arguments = arguments
                .iter()
                .map(|typ| convert_unnamed_type(typ, context))
                .map(|arg| {
                    extras.extend(arg.extras);
                    arg.typ
                })
                .reduce(|a, b| format!("{a}, {b}"));
            let typ = match arguments {
                Some(arguments) => format!("{typ}<{arguments}>"),
                None => typ,
            };
            let typ = format!("type {name} = {typ}");
            WithExtraTypes { typ, extras }
        }
        Type::Struct(struct_t) => convert_struct(name, struct_t, context),
        _ => unimplemented![],
    }
}

fn convert_unnamed_type(typ: &Type, context: &mut Context) -> WithExtraTypes {
    use gosyn::ast::Type;
    match typ {
        Type::Name(name) => WithExtraTypes {
            typ: convert_name(name),
            extras: vec![],
        },
        Type::Instantiated(InstantiatedType {
            name, arguments, ..
        }) => {
            let mut extras = vec![];
            let typ = convert_name(name);
            let arguments = arguments
                .iter()
                .map(|typ| convert_unnamed_type(typ, context))
                .map(|arg| {
                    extras.extend(arg.extras);
                    arg.typ
                })
                .reduce(|a, b| format!("{a}, {b}"));
            let typ = match arguments {
                Some(arguments) => format!("{typ}<{arguments}>"),
                None => typ,
            };
            WithExtraTypes { typ, extras }
        }
        Type::Struct(struct_t) => {
            let name = generate_type_name(AnonymousType::Struct(struct_t));
            convert_struct(&name, struct_t, context)
        }
        Type::Slice(SliceType { typ, .. }) => {
            let WithExtraTypes { typ, extras } = convert_unnamed_type(typ, context);
            let typ = format!("&[{typ}]");
            WithExtraTypes { extras, typ }
        }
        Type::Array(ArrayType { len, typ, .. }) => {
            let len = convert_expr(len, context);
            let WithExtraTypes { typ, extras } = convert_unnamed_type(typ, context);
            let typ = format!["{typ}[{len}]"];
            WithExtraTypes { extras, typ }
        }
        Type::Map(MapType { key, val, .. }) => {
            let WithExtraTypes {
                typ: key,
                mut extras,
            } = convert_unnamed_type(key, context);
            let WithExtraTypes {
                typ: val,
                extras: val_extras,
            } = convert_unnamed_type(val, context);
            extras.extend(val_extras);
            let typ = format!["HashMap<{key}, {val}>"];
            WithExtraTypes { extras, typ }
        }
        _ => unimplemented![],
    }
}

fn convert_name(name: &gosyn::ast::NameType) -> String {
    match name {
        gosyn::ast::NameType::Ident(ident) => ident.name.clone(),
        gosyn::ast::NameType::Qualified(qual_ident) => {
            format!(
                "{}::{}",
                qual_ident.package.name, qual_ident.identifier.name
            )
        }
    }
}

pub enum AnonymousType<'a> {
    Struct(&'a StructType),
    Interface(&'a InterfaceType),
}

fn generate_type_name(typ: AnonymousType) -> String {
    let count = next_anon_count();
    match typ {
        AnonymousType::Struct(_) => format!("AnonymousStruct{count}"),
        AnonymousType::Interface(_) => format!("AnonymousInterface{count}"),
    }
}

static ANON_TYPE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_anon_count() -> usize {
    ANON_TYPE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1
}

fn convert_struct(name: &str, typ: &StructType, context: &mut Context) -> WithExtraTypes {
    let mut extras = vec![];
    let header = format!("struct {name}{{");
    let mut fields = vec![];
    for field in &typ.fields {
        match &field.field {
            gosyn::ast::Field::Named(field) => {
                let WithExtraTypes {
                    typ,
                    extras: field_extras,
                } = convert_unnamed_type(&field.typ, context);
                extras.extend(field_extras);
                for name in &field.name {
                    fields.push(format!("{}: {},", &name.name, typ));
                }
            }
            gosyn::ast::Field::Embedded(field) => {
                let mut extras = vec![];
                let mut arguments = vec![];

                for arg in &field.type_args {
                    let WithExtraTypes {
                        typ,
                        extras: arg_extras,
                    } = convert_unnamed_type(arg, context);
                    extras.extend(arg_extras);
                    arguments.push(typ);
                }
                let arguments = if !arguments.is_empty() {
                    format!("<{}>", arguments.join(", "))
                } else {
                    String::new()
                };

                let name = match &field.typ {
                    gosyn::ast::NameType::Ident(name) => &name.name,
                    gosyn::ast::NameType::Qualified(name) => &name.identifier.name,
                };

                let typ = convert_name(&field.typ);

                fields.push(format!("{name}: {typ}{arguments},"));
            }
        };
    }
    let fields = fields.join("\n");
    let tail = "}";
    let typ = format!("{header}\n{fields}\n{tail}");
    WithExtraTypes { typ, extras }
}

pub fn convert_function(func: &FuncDecl, context: &mut Context) -> String {
    let name = &func.name.name;
    let type_params = vec![];
    let mut extras = vec![];
    for arg in &func.typ.typ_params {
        let WithExtraTypes {
            typ,
            extras: arg_extras,
        } = unimplemented![]; // convert_unnamed_type(&arg.constraint);
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
    for param in &func.typ.params.parameters {
        let WithExtraTypes {
            typ,
            extras: param_extras,
        } = convert_unnamed_type(&param.typ, context);
        extras.extend(param_extras);

        match param.identifiers.as_slice() {
            [] => {
                // We convert variadic parameters into vectors for now
                if param.variadic.is_some() {
                    params.push(format!("_: Vec<{typ}>"));
                } else {
                    params.push(format!("_: {typ}"));
                }
            }
            [name] => {
                let name = &name.name;
                // We convert variadic parameters into vectors for now
                if param.variadic.is_some() {
                    params.push(format!("{name}: Vec<{typ}>"));
                } else {
                    params.push(format!("{name}: {typ}"));
                }
            }
            names => {
                for (index, name) in names.iter().enumerate() {
                    let name = &name.name;
                    // We convert variadic parameters into vectors for now
                    if param.variadic.is_some() && index == names.len() - 1 {
                        params.push(format!("{name}: Vec<{typ}>"));
                    } else {
                        params.push(format!("{name}: {typ}"));
                    }
                }
            }
        };
    }

    let params = params.join(", ");

    let mut result = vec![];

    for res in &func.typ.result.parameters {
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

    if let Some(body) = &func.body {
        let statements = convert_block_stmt(body, context);
        // Note that `convert_block_stmt` wraps the block in braces so we don't need to wrap the body of the function in braces here
        format!("fn {name}{type_params}({params}){result}{statements}")
    } else {
        format!("unsafe extern \"C\" {{fn {name}{type_params}({params}){result};}}")
    }
}

pub fn convert_function_literal(func: &FuncLit, context: &mut Context) -> String {
    let mut extras = vec![];
    let mut params = vec![];
    for param in &func.typ.params.parameters {
        let WithExtraTypes {
            typ,
            extras: param_extras,
        } = convert_unnamed_type(&param.typ, context);
        extras.extend(param_extras);

        match param.identifiers.as_slice() {
            [] => {
                // We convert variadic parameters into vectors for now
                if param.variadic.is_some() {
                    params.push(format!("_: Vec<{typ}>"));
                } else {
                    params.push(format!("_: {typ}"));
                }
            }
            [name] => {
                let name = &name.name;
                // We convert variadic parameters into vectors for now
                if param.variadic.is_some() {
                    params.push(format!("{name}: Vec<{typ}>"));
                } else {
                    params.push(format!("{name}: {typ}"));
                }
            }
            names => {
                for (index, name) in names.iter().enumerate() {
                    let name = &name.name;
                    // We convert variadic parameters into vectors for now
                    if param.variadic.is_some() && index == names.len() - 1 {
                        params.push(format!("{name}: Vec<{typ}>"));
                    } else {
                        params.push(format!("{name}: {typ}"));
                    }
                }
            }
        };
    }

    let params = params.join(", ");

    let mut result = vec![];

    for res in &func.typ.result.parameters {
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

    let statements = convert_block_stmt(&func.body, context);
    // Note that `convert_block_stmt` wraps the block in braces so we don't need to wrap the body of the function in braces here
    format!("|{params}|{result}{statements}")
}

pub fn convert_selector(selector: &Selector, context: &mut Context) -> String {
    let left = convert_expr(&selector.x, context);
    let right = &selector.sel.name;
    format!("{left}.{right}")
}

pub fn convert_expr(expr: &Expression, context: &mut Context) -> String {
    match expr {
        Expression::Ident(ident) => ident.name.clone(),
        Expression::Operation(op) => {
            let x = convert_expr(&op.x, context);
            let y = convert_expr(op.y.as_ref().unwrap(), context);
            let op: &str = op.op.into();
            format!("{x} {op} {y}")
        }
        Expression::BasicLit(lit) => lit.value.clone(),
        Expression::Paren(expr) => {
            let expr = convert_expr(&expr.expr, context);
            format!("({expr})")
        }
        Expression::Call(call) => {
            // TODO Handle passing variadic arguments.
            // TODO Handle passing rest arguments.
            let func = convert_expr(&call.func, context);
            if call.dots.is_some() {
                unimplemented!("Cannot flatten rest arguments yet");
            }
            let args = call
                .args
                .iter()
                .map(|expr| convert_expr(expr, context))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{func}({args})")
        }
        Expression::FuncLit(func) => convert_function_literal(func, context),
        Expression::Selector(selector) => convert_selector(selector, context),
        _ => unimplemented![],
    }
}

pub fn convert_stmt(stmt: &Statement, context: &mut Context) -> String {
    match stmt {
        Statement::Expr(expr) => convert_expr(&expr.expr, context),
        Statement::Block(block) => convert_block_stmt(block, context),
        Statement::Return(ret) => match ret.ret.as_slice() {
            [] => "return;".to_string(),
            [expr] => {
                let expr = convert_expr(expr, context);
                format!("return {expr};")
            }
            exprs => {
                let exprs = exprs
                    .iter()
                    .map(|expr| convert_expr(expr, context))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("return ({exprs});")
            }
        },
        Statement::Assign(assign) => {
            // TODO We need to fix this to evaluate everything in the correct order.
            let left = assign
                .left
                .iter()
                .map(|expr| convert_expr(expr, context))
                .collect::<Vec<_>>();
            let right = assign
                .right
                .iter()
                .map(|expr| convert_expr(expr, context))
                .collect::<Vec<_>>();
            let op: &str = assign.op.into();
            left.into_iter()
                .zip(right)
                .map(|(left, right)| format!("{left} {op} {right}"))
                .collect::<Vec<_>>()
                .join(";\n")
        }
        Statement::Declaration(decl) => match decl {
            DeclStmt::Type(typ_decl) => convert_type_decl(typ_decl, context),
            DeclStmt::Const(const_decl) => convert_const_decl(const_decl, context),
            DeclStmt::Variable(var_decl) => convert_variable_decl(var_decl, context),
        },
        Statement::ShortVarDecl(decl) => convert_short_variable_decl(decl, context),
        Statement::If(if_stmt) => convert_if_stmt(if_stmt, context),
        _ => unimplemented![],
    }
}

fn convert_block_stmt(block: &BlockStmt, context: &mut Context) -> String {
    let statements = block
        .list
        .iter()
        .map(|stmt| convert_stmt(stmt, context))
        .collect::<Vec<_>>()
        .join(";\n");
    format!("{{\n{statements}\n}}")
}

fn convert_if_stmt(if_stmt: &IfStmt, context: &mut Context) -> String {
    let cond = convert_expr(&if_stmt.cond, context);
    let body = convert_block_stmt(&if_stmt.body, context);
    let else_ = if let Some(else_) = &if_stmt.else_ {
        let else_ = convert_stmt(else_, context);
        // The else statement is either an if statement or a block statement so the formatting will work the same in rust as it will for go with braces and we don't need to do anything special.
        format!(" else {else_}")
    } else {
        String::new()
    };

    if let Some(init) = &if_stmt.init {
        let init = convert_stmt(init, context);

        // We must put the init statement and following if block in it's own scope as variables declared in go init statements are local to the scope of the if statement they are declared for, hence the extra surrounding braces.
        format!("{{\n{init}\nif {cond} {body}{else_}\n}}")
    } else {
        format!("if {cond} {body}{else_}")
    }
}

#[cfg(test)]
mod tests {
    use gosyn::Parser;

    use crate::{Context, convert_ast};

    /// Transforms the golang string into rust then asserts that it is exactly equal to the given rust string.
    fn assert_transformation(golang: &str, rust: &str) {
        let mut parser = Parser::from(golang);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
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
            "fn test(arg: first[5]){\n\n}",
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
            "fn test(){\nlet mut (ok, err) = calculate();\n}",
        );
    }

    #[test]
    fn transform_short_variable_declaration_with_multi_name_and_expression() {
        assert_transformation(
            "package main;\nfunc test (){ok, err:=2, 5}",
            "fn test(){\nlet mut (ok, err) = (2, 5);\n}",
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
            "fn test(){\n{\nlet mut (a, b) = calc();\nif a > b {\ndo_thing()\n} else {\ndo_other_thing()\n}\n}\n}",
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
}
