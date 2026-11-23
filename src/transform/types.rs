use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ast::{
    ArrayType, Field, FuncType, InstantiatedType, InterfaceElem, InterfaceType, MapType, NameType,
    SliceType, StructType, Type, TypeElem, TypeTerm,
};

use crate::transform::convert_function_signature;
use crate::{name_resolution::Context, transform::expressions::convert_expr};

/// This is used to hold type declarations that cannot be inserted in place but that need to be inserted somewhere.
/// This is needed because Go allows for nameless structs and interfaces but rust does not so we must give them names and declare them separately then reference their names in place.
pub struct WithExtraTypeDecls {
    /// Represents the current code to be inserted in place
    pub typ: String,
    /// Represents code to be inserted not in the current position
    pub extras: Vec<String>,
}

impl WithExtraTypeDecls {
    /// This appends all of the current extras into the passed `Vec` and returns the inline name.
    /// This is typically used in contexts where multiple `WithExtraTypedecls` are being returned from functions and all of the extras must be collected into a single `Vec` to be returned later on.
    fn add_to(self, extras: &mut Vec<String>) -> String {
        extras.extend(self.extras);
        self.typ
    }
}

/// Returns the rust representation of a type declaration for the specified type with the given name along with any anonymus types that were made while making the passed type.
/// The anonymus type declarations appear first in the returned `Vec` with the one with the passed name being last.
pub fn convert_named_type(name: &str, typ: &Type, context: &mut Context) -> Vec<String> {
    match typ {
        Type::Name(t_name) => {
            let typ = convert_name(t_name);
            vec![format!("type {name} = {typ};")]
        }
        Type::Instantiated(InstantiatedType {
            name: t_name,
            arguments,
        }) => {
            let mut extras = vec![];
            let typ = convert_name(t_name);
            let arguments = arguments
                .into_iter()
                .map(|typ| convert_unnamed_type(typ, context).add_to(&mut extras))
                .collect::<Vec<_>>()
                .join(", ");
            let typ = if arguments.len() > 0 {
                format!("{typ}<{arguments}>")
            } else {
                format!("{typ}")
            };
            let typ = format!("type {name} = {typ};");
            extras.push(typ);
            extras
        }
        Type::Struct(struct_t) => convert_struct(name, struct_t, context),
        Type::Pointer(ptr) => {
            // We handle this separately so we can add in lifetime anotations.
            let WithExtraTypeDecls { typ, mut extras } = convert_unnamed_type(&ptr.typ, context);
            extras.push(format!("type {name}<'a> = &'a {typ};"));
            extras
        }
        Type::Slice(slice) => {
            // We handle this separately so we can add in lifetime anotations, same as above.
            let WithExtraTypeDecls { typ, mut extras } = convert_unnamed_type(&slice.typ, context);
            extras.push(format!("type {name}<'a> = &'a [{typ}];"));
            extras
        }
        Type::Interface(interface_type) => convert_interface(name, interface_type, context),
        other => {
            // There is nothing special about the remaining types so we can use normal rust type declaration syntax for them and not provide lifetime anotations.
            // TODO Investigate making these not just type aliases.
            let WithExtraTypeDecls { typ, mut extras } = convert_unnamed_type(other, context);
            extras.push(format!("type {name} = {typ};"));
            extras
        }
    }
}

fn convert_interface(
    name: &str,
    interface_type: &InterfaceType,
    context: &mut Context,
) -> Vec<String> {
    let type_elements = interface_type
        .elements
        .iter()
        .filter_map(|el| match el {
            InterfaceElem::Type(el) => Some(el),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut extras = vec![];
    let requirements = match type_elements.as_slice() {
        [] => "".to_string(),
        other => panic!(
            "Cannot transform interfaces with type elements, in interface {name} got type elements: {other:?}"
        ),
    };
    let methods = interface_type
        .elements
        .iter()
        .filter_map(|el| match el {
            InterfaceElem::Method(el) => Some(el),
            _ => None,
        })
        .map(|method| {
            convert_function_signature(method.name.name.as_str(), &None, &method.typ, context) + ";"
        })
        .collect::<Vec<_>>()
        .join("\n");
    let typ = format!("trait {name}{requirements} {{\n{methods}\n}}");
    extras.push(typ);
    extras
}

pub fn convert_unnamed_type(typ: &Type, context: &mut Context) -> WithExtraTypeDecls {
    match typ {
        Type::Name(name) => WithExtraTypeDecls {
            typ: convert_name(name),
            extras: vec![],
        },
        Type::Instantiated(InstantiatedType { name, arguments }) => {
            let mut extras = vec![];
            let typ = convert_name(name);
            let arguments = arguments
                .iter()
                .map(|typ| convert_unnamed_type(typ, context).add_to(&mut extras))
                .collect::<Vec<_>>()
                .join(", ");
            let typ = format!("{typ}<{arguments}>");
            WithExtraTypeDecls { typ, extras }
        }
        Type::Struct(struct_t) => {
            let name = generate_type_name(AnonymousType::Struct(struct_t));
            let extras = convert_struct(&name, struct_t, context);
            WithExtraTypeDecls { extras, typ: name }
        }
        Type::Slice(SliceType { typ }) => {
            let WithExtraTypeDecls { typ, extras } = convert_unnamed_type(typ, context);
            let typ = format!("&[{typ}]");
            WithExtraTypeDecls { extras, typ }
        }
        Type::Array(ArrayType { len, typ, .. }) => {
            let len = convert_expr(len, context);
            let WithExtraTypeDecls { typ, extras } = convert_unnamed_type(typ, context);
            let typ = format!["[{typ}; {len}]"];
            WithExtraTypeDecls { extras, typ }
        }
        Type::Map(map_type) => convert_map_type(map_type, context),
        Type::Pointer(pointer_type) => {
            let WithExtraTypeDecls { typ, extras } =
                convert_unnamed_type(&pointer_type.typ, context);
            let typ = format!("&mut {typ}");
            WithExtraTypeDecls { typ, extras }
        }
        Type::Paren(paren_type) => {
            let WithExtraTypeDecls { typ, extras } = convert_unnamed_type(&paren_type.typ, context);
            let typ = format!("({typ})");
            WithExtraTypeDecls { typ, extras }
        }
        Type::Function(func_type) => convert_func_type(func_type, context),
        Type::Channel(channel_type) => {
            let WithExtraTypeDecls { typ, extras } =
                convert_unnamed_type(&channel_type.typ, context);
            // Not strictly proper but good enough I think.
            let typ = format!("chan<{typ}>");
            WithExtraTypeDecls { typ, extras }
        }
        Type::Interface(interface_type) => {
            let name = generate_type_name(AnonymousType::Interface(interface_type));
            let extras = convert_interface(&name, interface_type, context);
            let typ = format!("impl {name}");
            WithExtraTypeDecls { typ, extras }
        }
    }
}

pub fn convert_func_type(func_type: &FuncType, context: &mut Context) -> WithExtraTypeDecls {
    let mut extras = vec![];

    let mut params = vec![];
    for param in &func_type.params {
        let WithExtraTypeDecls {
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
        let WithExtraTypeDecls {
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

    let typ = format!("Box<dyn Fn({params}){result}>");
    WithExtraTypeDecls { typ, extras }
}

fn convert_map_type(map_type: &MapType, context: &mut Context) -> WithExtraTypeDecls {
    let MapType { key, val, .. } = map_type;
    let WithExtraTypeDecls {
        typ: key,
        mut extras,
    } = convert_unnamed_type(&key, context);
    let WithExtraTypeDecls {
        typ: val,
        extras: val_extras,
    } = convert_unnamed_type(val, context);
    extras.extend(val_extras);
    let typ = format!["HashMap<{key}, {val}>"];
    WithExtraTypeDecls { extras, typ }
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

fn convert_struct(name: &str, typ: &StructType, context: &mut Context) -> Vec<String> {
    let mut decls = vec![];
    let header = format!("struct {name}");
    let mut fields = vec![];
    for field in &typ.fields {
        match &field.field {
            Field::Named(field) => {
                let typ = convert_unnamed_type(&field.typ, context).add_to(&mut decls);
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
                let mut arguments = vec![];

                for arg in &field.type_args {
                    let typ = convert_unnamed_type(arg, context).add_to(&mut decls);
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
    let typ = format!("{header}{{\n{fields}\n}}");
    decls.push(typ);
    decls
}

pub fn convert_type_element(constraint: &TypeElem, context: &mut Context) -> WithExtraTypeDecls {
    match constraint.types.as_slice() {
        [constraint] => match constraint {
            TypeTerm::Type(typ) => convert_unnamed_type(typ, context),
            // TODO Maybe implement these as `AsRef<Type>`??
            other => unimplemented![
                "Underlying type constraints are not currently supported, got: {other:?}"
            ],
        },
        other => unimplemented![
            "Generic type constraints with more then 1 union type are not supported, got: {other:?}",
        ],
    }
}
