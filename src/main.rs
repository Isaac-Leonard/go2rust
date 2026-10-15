pub mod name_resolution;

use std::collections::{HashMap, hash_map::Entry};
use std::sync::atomic::{AtomicUsize, Ordering};

use gosyn::ast::{Declaration, Expression, File, Import, InstantiatedType, Statement, StructType};
use gosyn::ast::{FuncDecl, InterfaceType};
use gosyn::token::Operator;
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
    let mut context = Context::default();
    let imports = file
        .imports
        .iter()
        .map(|import| convert_import(import, &mut context))
        .try_fold(String::new(), |a, b| -> Result<String, String> {
            Ok(a + &b?)
        })?;

    let declarations = file
        .decl
        .iter()
        .map(|decl| convert_decl(decl, &mut context))
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
        Declaration::Type(type_d) => {
            let mut types = String::new();
            for spec in &type_d.specs {
                context
                    .types
                    .insert(spec.name.name.clone(), spec.typ.clone());
                types += &convert_named_type(&spec.name.name, &spec.typ).typ;
            }
            Ok(types)
        }
        Declaration::Function(func_decl) => Ok(convert_function(func_decl, context)),
        Declaration::Const(decl) => todo!(),
        Declaration::Variable(decl) => todo!(),
    }
}

pub struct WithExtraTypes {
    typ: String,
    extras: Vec<String>,
}

fn convert_named_type(name: &str, typ: &gosyn::ast::Type) -> WithExtraTypes {
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
                .map(convert_unnamed_type)
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
        Type::Struct(struct_t) => convert_struct(name, struct_t),
        _ => unimplemented![],
    }
}

fn convert_unnamed_type(typ: &gosyn::ast::Type) -> WithExtraTypes {
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
                .map(convert_unnamed_type)
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
            convert_struct(&name, struct_t)
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

fn convert_struct(name: &str, typ: &StructType) -> WithExtraTypes {
    let mut extras = vec![];
    let header = format!("struct {name}{{");
    let mut fields = vec![];
    for field in &typ.fields {
        match &field.field {
            gosyn::ast::Field::Named(field) => {
                let WithExtraTypes {
                    typ,
                    extras: field_extras,
                } = convert_unnamed_type(&field.typ);
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
                    } = convert_unnamed_type(arg);
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
        } = convert_unnamed_type(&param.typ);
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
        } = convert_unnamed_type(&res.typ);
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
        let mut statements: Vec<String> = vec![];
        for stmt in &body.list {
            statements.push(convert_stmt(stmt, context));
        }
        let statements = statements.join(";\n");
        format!("fn {name}{type_params}({params}){result}{{{statements}}}")
    } else {
        format!("unsafe extern \"C\" {{fn {name}{type_params}({params}){result};}}")
    }
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
        _ => unimplemented![],
    }
}

pub fn convert_stmt(stmt: &Statement, context: &mut Context) -> String {
    match stmt {
        Statement::Expr(expr) => convert_expr(&expr.expr, context),
        _ => unimplemented![],
    }
}

#[cfg(test)]
mod tests {
    use gosyn::Parser;

    use crate::{Context, convert_ast};

    #[test]
    fn transform_empty_struct_type() {
        let struct_type_src = "package main;\ntype test struct {}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "struct test{\n\n}");
    }

    #[test]
    fn transform_struct_type_with_named_fields() {
        let struct_type_src = "package main;\ntype test struct {first i64; second string}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "struct test{\nfirst: i64,\nsecond: string,\n}");
    }

    #[test]
    fn transform_struct_type_with_named_fields_with_generic() {
        let struct_type_src = "package main;\ntype test struct {first i64; second t[string]}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "struct test{\nfirst: i64,\nsecond: t<string>,\n}");
    }

    #[test]
    fn transform_struct_type_with_named_fields_with_multi_generics() {
        let struct_type_src = "package main;\ntype test struct {first i64; second t[string,i32, fourth[fifth,sixth]]}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(
            res,
            "struct test{\nfirst: i64,\nsecond: t<string, i32, fourth<fifth, sixth>>,\n}"
        );
    }

    #[test]
    fn transform_struct_type_with_unnamed_fields() {
        let struct_type_src = "package main;\ntype test struct {i64; string}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "struct test{\ni64: i64,\nstring: string,\n}");
    }

    #[test]
    fn transform_struct_type_with_qualified_unnamed_fields() {
        let struct_type_src = "package main;\ntype test struct {i64; t.second}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "struct test{\ni64: i64,\nsecond: t::second,\n}");
    }

    #[test]
    fn transform_struct_type_with_unnamed_fields_with_generics() {
        let struct_type_src = "package main;\ntype test struct {i64; t[third]}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "struct test{\ni64: i64,\nt: t<third>,\n}");
    }

    #[test]
    fn transform_struct_type_with_unnamed_fields_with_multi_generics() {
        let struct_type_src =
            "package main;\ntype test struct {i64; t[third, fourth, fifth.sixth]}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(
            res,
            "struct test{\ni64: i64,\nt: t<third, fourth, fifth::sixth>,\n}"
        );
    }

    #[test]
    fn transform_struct_type_with_qualified_unnamed_fields_with_generics() {
        let struct_type_src = "package main;\ntype test struct {i64; t.second[third]}";
        let mut parser = Parser::from(struct_type_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "struct test{\ni64: i64,\nsecond: t::second<third>,\n}");
    }

    #[test]
    fn transform_func_decl_with_no_params_or_result() {
        let func_src = "package main;\nfunc test () {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test(){}");
    }

    #[test]
    fn transform_func_decl_with_single_named_params_and_no_result() {
        let func_src = "package main;\nfunc test (count i64) {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test(count: i64){}");
    }

    #[test]
    fn transform_func_decl_with_multi_name_single_type_params_and_no_result() {
        let func_src = "package main;\nfunc test (a, b i64) {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test(a: i64, b: i64){}");
    }

    #[test]
    fn transform_func_decl_with_multi_name_multi_type_params_and_no_result() {
        let func_src = "package main;\nfunc test (first string, second i64) {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test(first: string, second: i64){}");
    }

    #[test]
    fn transform_func_decl_with_no_params_and_single_unnamed_result() {
        let func_src = "package main;\nfunc test ()string {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test()->string{}");
    }

    #[test]
    fn transform_func_decl_with_no_params_and_multi_unnamed_result() {
        let func_src = "package main;\nfunc test ()(string,error) {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test()->(string, error){}");
    }

    #[test]
    fn transform_func_decl_with_no_params_and_single_named_result() {
        let func_src = "package main;\nfunc test ()(name string) {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test()->string{}");
    }

    #[test]
    fn transform_func_decl_with_no_params_and_multi_named_single_type_result() {
        let func_src = "package main;\nfunc test ()(first, last string) {}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test()->(string, string){}");
    }

    #[test]
    fn transform_func_decl_with_no_params_or_result_or_body() {
        let func_src = "package main;\nfunc test ()";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "unsafe extern \"C\" {fn test();}");
    }

    #[test]
    fn transform_func_decl_with_params_and_result_with_simple_binary_expression() {
        let func_src = "package main;\nfunc test (a, b int)int{a+b}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test(a: int, b: int)->int{a + b}");
    }

    #[test]
    fn transform_func_decl_with_and_result_with_simple_binary_expression_on_int_literals() {
        let func_src = "package main;\nfunc test ()int{1+2}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test()->int{1 + 2}");
    }

    #[test]
    fn transform_func_decl_with_and_result_with_simple_binary_expression_on_string_literals() {
        let func_src = "package main;\nfunc test ()int{\"hello\" + \"world\"}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test()->int{\"hello\" + \"world\"}");
    }

    #[test]
    fn transform_func_decl_with_result_with_binary_expression_with_parens() {
        let func_src = "package main;\nfunc test ()int{1+(2+3)}";
        let mut parser = Parser::from(func_src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        let res = convert_ast(file, &mut Context::default()).unwrap();
        assert_eq!(res, "fn test()->int{1 + (2 + 3)}");
    }
}
