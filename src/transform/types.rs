use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ast::{
    ArrayType, Field, FuncType, InstantiatedType, InterfaceType, MapType, NameType, SliceType,
    StructType, Type, TypeElem, TypeTerm,
};

use crate::{name_resolution::Context, transform::expressions::convert_expr};

pub struct WithExtraTypes {
    pub typ: String,
    pub extras: Vec<String>,
}

pub fn convert_named_type(name: &str, typ: &Type, context: &mut Context) -> WithExtraTypes {
    match typ {
        Type::Name(t_name) => WithExtraTypes {
            typ: format!("type {} = {};", name, convert_name(t_name)),
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
                None => format!("{typ}"),
            };
            let typ = format!("type {name} = {typ};");
            WithExtraTypes { typ, extras }
        }
        Type::Struct(struct_t) => convert_struct(name, struct_t, context),
        Type::Map(map_type) => {
            let WithExtraTypes { typ, extras } = convert_map_type(map_type, context);
            let typ = format!("type {name} = {typ};");
            WithExtraTypes { extras, typ }
        }
        other => {
            unimplemented!["Not all types can be transformed yet, failed to transform {other:?}"]
        }
    }
}

pub fn convert_unnamed_type(typ: &Type, context: &mut Context) -> WithExtraTypes {
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
        Type::Map(map_type) => convert_map_type(map_type, context),
        Type::Pointer(pointer_type) => {
            let WithExtraTypes { typ, extras } = convert_unnamed_type(&pointer_type.typ, context);
            let typ = format!("&mut {typ}");
            WithExtraTypes { typ, extras }
        }
        Type::Paren(paren_type) => {
            let WithExtraTypes { typ, extras } = convert_unnamed_type(&paren_type.typ, context);
            let typ = format!("({typ})");
            WithExtraTypes { typ, extras }
        }
        Type::Function(func_type) => convert_func_type(func_type, context),
        Type::Channel(channel_type) => todo!(),
        Type::Interface(interface_type) => todo!(),
    }
}

pub fn convert_func_type(func_type: &FuncType, context: &mut Context) -> WithExtraTypes {
    let mut extras = vec![];

    let mut params = vec![];
    for param in &func_type.params {
        let WithExtraTypes {
            typ,
            extras: param_extras,
        } = convert_unnamed_type(&param.typ, context);
        extras.extend(param_extras);

        // Rust Fn types don't take name parameters so we only use the types here but we still must check each variant
        match param.identifiers.as_slice() {
            [] | [_] => {
                // We convert variadic parameters into vectors for now
                if param.variadic {
                    params.push(format!("Vec<{typ}>"));
                } else {
                    params.push(format!("{typ}"));
                }
            }
            names => {
                for (index, _) in names.iter().enumerate() {
                    // We convert variadic parameters into vectors for now
                    if param.variadic && index == names.len() - 1 {
                        params.push(format!("Vec<{typ}>"));
                    } else {
                        params.push(format!("{typ}"));
                    }
                }
            }
        };
    }
    let params = params.join(", ");

    let mut result = vec![];

    for res in &func_type.result {
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

    let typ = format!("Fn({params}){result}");
    WithExtraTypes { typ, extras }
}

fn convert_map_type(map_type: &MapType, context: &mut Context) -> WithExtraTypes {
    let MapType { key, val, .. } = map_type;
    let WithExtraTypes {
        typ: key,
        mut extras,
    } = convert_unnamed_type(&key, context);
    let WithExtraTypes {
        typ: val,
        extras: val_extras,
    } = convert_unnamed_type(val, context);
    extras.extend(val_extras);
    let typ = format!["HashMap<{key}, {val}>"];
    WithExtraTypes { extras, typ }
}

fn convert_name(name: &NameType) -> String {
    match name {
        NameType::Ident(ident) => ident.name.clone(),
        NameType::Qualified(qual_ident) => {
            format!(
                "{}::{}",
                qual_ident.package.name, qual_ident.identifier.name
            )
        }
    }
}

enum AnonymousType<'a> {
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
            Field::Named(field) => {
                let WithExtraTypes {
                    typ,
                    extras: field_extras,
                } = convert_unnamed_type(&field.typ, context);
                extras.extend(field_extras);
                for name in &field.name {
                    let name = match name.name.as_str() {
                        // _ is not a valid field name in rust so we must replace it.
                        // This is hacky but should work for now.
                        // TODO fix this
                        "_" => "__",
                        name => name,
                    };
                    fields.push(format!("{name}: {typ},",));
                }
            }
            Field::Embedded(field) => {
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
                    NameType::Ident(name) => &name.name,
                    NameType::Qualified(name) => &name.identifier.name,
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

pub fn convert_type_element(constraint: &TypeElem, context: &mut Context) -> WithExtraTypes {
    match constraint.types.as_slice() {
        [constraint] => match constraint {
            TypeTerm::Type(typ) => convert_unnamed_type(typ, context),
            // TODO Maybe implement these as `AsRef<Type>`??
            _ => unimplemented!["Underlying type constraints are not currently supported"],
        },
        _ => unimplemented![
            "Generic type constraints with more then 1 union type are not supported, got {} types in type element",
            constraint.types.len()
        ],
    }
}
