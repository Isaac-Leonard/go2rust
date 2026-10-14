pub mod name_resolution;

use std::collections::{HashMap, hash_map::Entry};
use std::sync::atomic::{AtomicUsize, Ordering};

use gosyn::ast::InterfaceType;
use gosyn::ast::{Declaration, File, Import, InstantiatedType, StructType};
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
        Declaration::Function(func_decl) => todo!(),
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
            arguements,
            ..
        }) => {
            let mut extras = vec![];
            let typ = convert_name(t_name);
            let arguments = arguements
                .iter()
                .map(convert_unnamed_type)
                .map(|arg| {
                    extras.extend(arg.extras);
                    arg.typ
                })
                .reduce(|a, b| format!("{a}, {b}"));
            let typ = match arguments {
                Some(arguements) => format!("{typ}<{arguements}>"),
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
            name, arguements, ..
        }) => {
            let mut extras = vec![];
            let typ = convert_name(name);
            let arguments = arguements
                .iter()
                .map(convert_unnamed_type)
                .map(|arg| {
                    extras.extend(arg.extras);
                    arg.typ
                })
                .reduce(|a, b| format!("{a}, {b}"));
            let typ = match arguments {
                Some(arguements) => format!("{typ}<{arguements}>"),
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
}
