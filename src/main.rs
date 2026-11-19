pub mod ast;
pub mod name_resolution;
pub mod type_inferance;
pub mod utils;

use std::sync::atomic::{AtomicUsize, Ordering};

use ast::{
    ArrayType, BasicLit, BlockStmt, BranchStmt, Call, CompositeLit, ConstSpec, Decl, DeclStmt,
    Declaration, Element, ExprSwitchStmt, Expression, Field, File, ForStmt, FuncDecl, FuncLit,
    FuncType, IfStmt, Import, Index, InstantiatedType, InterfaceType, KeyedElement, LiteralValue,
    MapType, NameType, RangeStmt, Selector, ShortVarDecl, SliceType, Statement, StructType,
    SwitchStmt, Type, TypeElem, TypeSpec, TypeTerm, VarSpec,
};
use gosyn::token::{Keyword, LitKind};
use name_resolution::Context;
use type_inferance::infer_type;

fn main() {
    let mut args = std::env::args();
    let Some(file_path) = args.nth(1) else {
        eprintln!("No file specified");
        return;
    };

    let dest = args.next();
    let ast = match gosyn::parse_file(file_path) {
        Ok(ast) => ast,
        Err(err) => {
            eprintln!("Failed to parse file, got error: {err}");
            return;
        }
    };

    match convert_ast(ast.into(), &mut Context::default()) {
        Ok(converted) => {
            if let Some(dest) = dest {
                if let Err(err) = std::fs::write(dest, converted) {
                    eprintln!("Failed to write file: got err {err}");
                    return;
                };
            } else {
                println!("{converted}");
            }
        }
        Err(err) => {
            eprintln!("Could not convert file, got error: {err}");
        }
    };
}

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
        [name] => match name.as_str() {
            // Rust does not let us have mut on _ bindings
            "_" => name.to_string(),
            _ => format!("mut {name}"),
        },
        names => {
            let names = names
                .iter()
                .map(|name| match name.as_str() {
                    // Rust does not let us have mut on _ bindings
                    "_" => name.to_string(),
                    _ => format!("mut {name}"),
                })
                .collect::<Vec<_>>()
                .join(", ");
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

    format!("let {names} = {values};")
}

pub struct WithExtraTypes {
    typ: String,
    extras: Vec<String>,
}

fn convert_named_type(name: &str, typ: &Type, context: &mut Context) -> WithExtraTypes {
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

fn convert_unnamed_type(typ: &Type, context: &mut Context) -> WithExtraTypes {
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

pub fn convert_function(func: &FuncDecl, context: &mut Context) -> String {
    let name = &func.name.name;
    let mut type_params = vec![];
    let mut extras = vec![];
    for arg in &func.typ.typ_params {
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
    for param in &func.typ.params {
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
    if let Some(recv) = &func.recv {
        let name = &recv[0].identifiers.first();
        match name {
            None => params.insert(0, "&self".to_string()),
            Some(name) => params.insert(0, format!("{ }:&mut Self", &name.name)),
        }
    }
    let params = params.join(", ");

    let mut result = vec![];

    for res in &func.typ.result {
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

    let recv = &func.recv;

    let func = if let Some(body) = &func.body {
        let statements = convert_block_stmt(body, context);
        // Note that `convert_block_stmt` wraps the block in braces so we don't need to wrap the body of the function in braces here
        format!("fn {name}{type_params}({params}){result}{statements}")
    } else {
        format!("unsafe extern \"C\" {{fn {name}{type_params}({params}){result};}}")
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

fn convert_type_element(constraint: &TypeElem, context: &mut Context) -> WithExtraTypes {
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

pub fn convert_function_literal(func: &FuncLit, context: &mut Context) -> String {
    let mut extras = vec![];
    let mut params = vec![];
    for param in &func.typ.params {
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

    let params = params.join(", ");

    let mut result = vec![];

    for res in &func.typ.result {
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

pub fn convert_struct_literal(name: &str, values: &LiteralValue, context: &mut Context) -> String {
    let mut fields = vec![];
    // If we can't determine names for fields then we will call `::new` on the struct instead passing the given fields as arguments
    let mut unnamed_fields = false;
    for KeyedElement { key, val } in &values.values {
        // We really should be checking that this is an identifier but we assume we are converting valid code.
        let key = key.as_ref().map(|key| convert_element(key, context));
        let val = convert_element(val, context);
        if key.is_none() {
            unnamed_fields = true;
        }
        fields.push((key, val))
    }
    if unnamed_fields {
        let fields = fields
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>()
            .join(", ");

        format!("{name}::new({fields})")
    } else {
        let fields = fields
            .into_iter()
            .map(|(key, value)| format!("{}: {},", key.unwrap(), value))
            .collect::<Vec<_>>()
            .join("\n");

        format!("{name} {{\n{fields}\n}}")
    }
}

pub fn convert_map_literal(map: &MapType, val: &LiteralValue, context: &mut Context) -> String {
    let key_type = convert_unnamed_type(&map.key, context).typ;
    let val_type = convert_unnamed_type(&map.val, context).typ;
    let mut contents = vec![];
    for KeyedElement { key, val } in &val.values {
        let key = key.as_ref().expect("unkeyed map found");
        let key = convert_element(key, context);
        let val = convert_element(val, context);
        contents.push((key, val));
    }
    let contents = contents
        .into_iter()
        .map(|(key, value)| format!("map.insert({key}, {value});"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{{\nlet mut map = HashMap::<{key_type}, {val_type}>::new();\n{contents}\nmap\n}}")
}

pub fn convert_array_literal(arr: &ArrayType, val: &LiteralValue, context: &mut Context) -> String {
    let el_type = convert_unnamed_type(&arr.typ, context).typ;
    let len = convert_expr(&arr.len, context);
    let mut elements = vec![];
    for KeyedElement { key, val } in &val.values {
        if key.is_some() {
            unimplemented!("keyed array or slice literals are not supported")
        }

        let val = convert_element(val, context);
        elements.push(val);
    }
    let elements = elements.join(", ");
    format!("{{\nlet array: [{el_type}; {len}] = [{elements}];\narray\n}}")
}

pub fn convert_slice_literal(
    slice: &SliceType,
    val: &LiteralValue,
    context: &mut Context,
) -> String {
    let len = Expression::BasicLit(BasicLit {
        kind: LitKind::Integer,
        value: val.values.len().to_string(),
    });
    let array_type = ArrayType {
        len: Box::new(len),
        typ: slice.typ.clone(),
    };
    let backing_array = convert_array_literal(&array_type, val, context);
    format!("{backing_array}.as_slice()")
}

pub fn convert_composite_literal(lit: &CompositeLit, context: &mut Context) -> String {
    match &*lit.typ {
        Expression::Ident(name) => convert_struct_literal(&name.name, &lit.val, context),
        Expression::Type(Type::Map(map)) => convert_map_literal(map, &lit.val, context),
        Expression::Type(Type::Array(arr)) => convert_array_literal(arr, &lit.val, context),
        Expression::Type(Type::Slice(slice)) => convert_slice_literal(slice, &lit.val, context),
        other => unimplemented![
            "Not all composite literals can be transformed yet, failed to convert {other:?}"
        ],
    }
}

fn convert_element(element: &Element, context: &mut Context) -> String {
    match element {
        Element::Expr(expr) => convert_expr(expr, context),
        Element::LitValue(lit_value) => {
            let mut elements = vec![];
            let mut keyed = false;
            for KeyedElement { key, val } in &lit_value.values {
                let element = convert_element(val, context);
                let key = if let Some(key) = key {
                    keyed = true;
                    let key = convert_element(key, context);
                    format!("{key}: ")
                } else {
                    String::new()
                };
                elements.push(format!("{key}{element}"));
            }
            let elements = if keyed {
                elements.join(",\n")
            } else {
                elements.join(", ")
            };

            if keyed {
                format!("hash_map!{{\n{elements}\n}}")
            } else {
                format!("vec![{elements}]")
            }
        }
    }
}

pub fn convert_selector(selector: &Selector, context: &mut Context) -> String {
    let left = convert_expr(&selector.x, context);
    let right = &selector.sel.name;
    format!("{left}.{right}")
}

fn convert_index_expr(index: &Index, context: &mut Context) -> String {
    let left = convert_expr(&index.left, context);
    let index = convert_expr(&index.index, context);
    format!("{left}[{index}]")
}

pub fn convert_call(call: &Call, context: &mut Context) -> String {
    // TODO Handle passing variadic arguments.
    // TODO Handle passing rest arguments.
    // TODO handle builtins
    match &*call.func {
        //Expression::Ident(ident) if context.is_builtin(ident) => match ident.name {
        //     "clear" => convert_clear_call(call.args, context),
        // },
        _ => {
            let func = convert_expr(&call.func, context);
            if call.dots {
                // unimplemented!("Cannot flatten rest arguments yet");
            }
            let args = call
                .args
                .iter()
                .map(|expr| convert_expr(expr, context))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{func}({args})")
        }
    }
}

fn convert_clear_call(args: &[Expression], context: &mut Context) -> String {
    match args {
        [arg] => {
            let typ = infer_type(arg, context);
            let arg = convert_expr(arg, context);
            match typ {
                Type::Map(_) => format!("({arg}).clear()"),
                Type::Slice(_) => todo![],
                Type::Name(name_type) => todo!(),
                Type::Instantiated(instantiated_type) => todo!(),
                Type::Array(array_type) => todo!(),
                Type::Function(func_type) => todo!(),
                Type::Struct(struct_type) => todo!(),
                Type::Channel(channel_type) => todo!(),
                Type::Pointer(pointer_type) => todo!(),
                Type::Interface(interface_type) => todo!(),
                Type::Paren(paren_type) => todo!(),
            }
        }
        _ => unreachable![],
    }
}

pub fn convert_expr(expr: &Expression, context: &mut Context) -> String {
    match expr {
        Expression::Ident(ident) => ident.name.clone(),
        Expression::Operation(op) => {
            let x = convert_expr(&op.x, context);
            let y = op.y.as_ref().map(|y| convert_expr(y, context));
            let op: &str = op.op.into();
            match y {
                Some(y) => format!("{x} {op} {y}"),
                None => format!("{op}{x}"),
            }
        }
        Expression::BasicLit(lit) => lit.value.clone(),
        Expression::Paren(expr) => {
            let expr = convert_expr(&expr.expr, context);
            format!("({expr})")
        }
        Expression::Call(call) => convert_call(call, context),
        Expression::FuncLit(func) => convert_function_literal(func, context),
        Expression::Selector(selector) => convert_selector(selector, context),
        Expression::Index(index) => convert_index_expr(index, context),
        Expression::CompositeLit(lit) => convert_composite_literal(lit, context),
        Expression::Star(pointer) => {
            let expr = convert_expr(&pointer.right, context);
            format!("*{expr}")
        }
        Expression::IndexList(index_list) => todo!(),
        Expression::Slice(slice) => todo!(),
        Expression::Ellipsis(ellipsis) => todo!(),
        Expression::Range(range_expr) => todo!(),
        Expression::TypeAssert(type_assertion) => todo!(),
        Expression::List(expressions) => todo!(),
        Expression::Type(typ) => convert_unnamed_type(typ, context).typ,
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
        Statement::For(for_stmt) => convert_for_stmt(for_stmt, context),
        Statement::Range(range_stmt) => convert_range_stmt(range_stmt, context),
        Statement::Empty(_) => "{}".to_string(),
        Statement::Branch(branch) => convert_branch_stmt(branch, context),
        Statement::Switch(SwitchStmt::Expr(expr_switch)) => {
            convert_expr_switch_stmt(expr_switch, context)
        }
        other => unimplemented!["Cannot yet convert {other:?}"],
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

fn convert_for_stmt(for_stmt: &ForStmt, context: &mut Context) -> String {
    let (loop_keyword, condition) = if let Some(cond) = &for_stmt.cond {
        ("while", convert_stmt(cond, context))
    } else {
        ("loop", String::new())
    };

    let mut body = convert_block_stmt(&for_stmt.body, context);
    if let Some(post) = &for_stmt.post {
        let post = convert_stmt(post, context);
        // We run the post condition at the end of the body, it's not as clean but it does the same job
        // TODO Check how this will work with break statements
        body = format!("{{\n{body}\n{post}\n}}");
    }

    let loop_stmt = format!("{loop_keyword} {condition}{body}");

    if let Some(init) = &for_stmt.init {
        let init = convert_stmt(init, context);
        // We must put the init statement and following for block in it's own scope as variables declared in go init statements are local to the scope of the for statement they are declared for, hence the extra surrounding braces.
        format!("{{\n{init}\n{loop_stmt}\n}}")
    } else {
        loop_stmt
    }
}

fn convert_range_stmt(range_stmt: &RangeStmt, context: &mut Context) -> String {
    let key = range_stmt
        .key
        .as_ref()
        .map(|key| convert_expr(key, context));

    let val = range_stmt
        .value
        .as_ref()
        .map(|val| convert_expr(val, context));

    let decl = match (key, val) {
        (Some(key), Some(val)) => format!("({key}, {val})"),
        (Some(var), None) | (None, Some(var)) => var,
        (None, None) => panic!("Cannot have range loop without a key or value"),
    };

    let mut expr = convert_expr(&range_stmt.expr, context);
    let mut body = convert_block_stmt(&range_stmt.body, context);

    // TODO We need to find a way to assign the variables outside of the loop scope if a simple "=" operator is used instead of ":=" for the range expression.
    format!("for {decl} in {expr} {body}")
}

fn convert_branch_stmt(branch: &BranchStmt, context: &mut Context) -> String {
    let label = branch
        .ident
        .as_ref()
        .map(|label| format!(" '{}", label.name))
        .unwrap_or_default();
    let control_flow = match &branch.key {
        Keyword::Continue => "continue",
        Keyword::Break => "break",
        other => panic!("Unexpected keyword {other:?} found in branch statement"),
    };

    format!("{control_flow}{label};")
}

fn convert_expr_switch_stmt(switch: &ExprSwitchStmt, context: &mut Context) -> String {
    let convert_to_if = switch.tag.is_none()
        || !switch.block.body.iter().all(|case| {
            case.list
                .iter()
                .all(|expr| matches!(expr, Expression::Ident(_) | Expression::BasicLit(_)))
        });
    let init = switch.init.as_ref().map(|init| convert_stmt(init, context));
    let switch = if convert_to_if {
        let tag = switch
            .tag
            .as_ref()
            .map(|expr| convert_expr(expr, context))
            .unwrap_or_else(|| "true".to_string());
        let mut cases = vec![];
        for case in &switch.block.body {
            let mut conditions = case
                .list
                .iter()
                .map(|cond| convert_expr(cond, context))
                .map(|cond| format!("({cond}) == {tag}"))
                .collect::<Vec<_>>()
                .join(" || ");
            let block = case
                .body
                .iter()
                .map(|stmt| convert_stmt(stmt, context))
                .collect::<Vec<_>>()
                .join(";\n");
            let if_keyword = match case.tok {
                Keyword::Case => "if ",
                Keyword::Default => "",
                other => panic!("Found unexpected keyword used for switch statement: {other:?}",),
            };
            cases.push(format!("{if_keyword}{conditions} {{\n{block}\n}}"))
        }
        cases.join(" else ")
    } else {
        // Unwrap is safe here as we have already checked that tag exists when checking if we should convert to an if statement or not.
        let tag = switch.tag.as_ref().unwrap();
        let tag = convert_expr(tag, context);
        let mut cases = vec![];
        for case in &switch.block.body {
            let mut conditions = case
                .list
                .iter()
                .map(|cond| convert_expr(cond, context))
                .collect::<Vec<_>>()
                .join(" | ");
            let block = case
                .body
                .iter()
                .map(|stmt| convert_stmt(stmt, context))
                .collect::<Vec<_>>()
                .join(";\n");
            // We don't know what statement finishes the block and some, like return statements require a trailing semicolon in match branches so we take the simple route and just wrap all branches in braces.
            let block = format!("{{\n{block}\n}}");
            let conditions = match case.tok {
                Keyword::Case => conditions,
                Keyword::Default => "_".to_string(),
                other => panic!("Found unexpected keyword used for switch statement: {other:?}",),
            };
            cases.push(format!("{conditions} => {block}"))
        }
        let cases = cases.join("\n");
        format!("match {tag} {{\n{cases}\n}}")
    };
    if let Some(init) = init {
        format!("{{\n{init};\n{switch}\n}}")
    } else {
        switch
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
}
