use gosyn::token::LitKind;

use crate::{
    ast::{
        ArrayType, BasicLit, Call, CompositeLit, Element, Expression, FuncLit, Index, KeyedElement,
        LiteralValue, MapType, Selector, Slice, SliceType, Type,
    },
    name_resolution::Context,
    transform::{
        statements::convert_block_stmt,
        types::{WithExtraTypeDecls, convert_unnamed_type},
    },
};

fn convert_function_literal(func: &FuncLit, context: &mut Context) -> String {
    let mut extras = vec![];
    let mut params = vec![];
    for param in &func.typ.params {
        let WithExtraTypeDecls {
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

    let statements = convert_block_stmt(&func.body, context);
    // Note that `convert_block_stmt` wraps the block in braces so we don't need to wrap the body of the function in braces here
    format!("|{params}|{result}{statements}")
}

fn convert_struct_literal(name: &str, values: &LiteralValue, context: &mut Context) -> String {
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

fn convert_map_literal(map: &MapType, val: &LiteralValue, context: &mut Context) -> String {
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

fn convert_array_literal(arr: &ArrayType, val: &LiteralValue, context: &mut Context) -> String {
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

fn convert_slice_literal(slice: &SliceType, val: &LiteralValue, context: &mut Context) -> String {
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

fn convert_composite_literal(lit: &CompositeLit, context: &mut Context) -> String {
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

fn convert_selector(selector: &Selector, context: &mut Context) -> String {
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
        Expression::Slice(slice) => convert_slice(slice, context),
        // Just pass the variable for now as if it wasn't attemting to be flattened.
        // This generally works because it is generally used to pass to variadic functions where we also remove the rest parameter from so we end up just passing a vec to a vec in most cases hopefully.
        Expression::Ellipsis(ellipsis) => {
            if let Some(elt) = ellipsis.elt.as_deref() {
                convert_expr(elt, context)
            } else {
                "_".to_string()
            }
        }
        // I don't believe these can appear anywhere but in for loops which are handled separately leaving this variant as unused.
        Expression::Range(range_expr) => {
            unreachable!["Invalid assumption made about range expressions, got: {range_expr:?}."]
        }
        Expression::TypeAssert(type_assertion) => {
            panic!(
                "Attempted to convert type assertion but rust has no real equivalent, got: {type_assertion:?}"
            )
        }
        // This variant will be removed in future
        Expression::List(_expressions) => todo!(),
        Expression::Type(typ) => convert_unnamed_type(typ, context).typ,
    }
}

fn convert_slice(slice: &Slice, context: &mut Context) -> String {
    let left = convert_expr(&slice.left, context);
    let index = slice
        .index
        .each_ref()
        .map(|index| index.as_deref().map(|exp| convert_expr(exp, context)));

    let index = match index {
        [None, None, None] => "..".to_string(),
        [Some(start), None, None] => format!("{start}.."),
        [None, Some(end), None] => format!("..{end}"),
        [Some(start), Some(end), None] => format!("{start}..{end}"),
        other => {
            panic!("full slice expressions have no equivilent in rust, got: {other:?}")
        }
    };

    format!("{left}[{index}]")
}
