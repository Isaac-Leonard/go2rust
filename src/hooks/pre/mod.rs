use crate::{
    ast::{FuncType, Ident, InstantiatedType, NameType, PointerType, Type},
    name_resolution::Context,
};

pub fn upper_case_string_types(typ: &mut Type, _context: &mut Context) {
    let string_type_name = "String".to_string();
    if let Type::Name(NameType::Ident(Ident { name })) = typ
        && name.as_str() == "string"
    {
        *name = string_type_name.clone();
    }
}

pub fn return_slice_to_vec(func_type: &mut FuncType, _context: &mut Context) {
    for param in &mut func_type.result {
        match &param.typ {
            Type::Slice(inner) => {
                param.typ = Type::Instantiated(InstantiatedType {
                    name: NameType::Ident(Ident {
                        name: "Vec".to_string(),
                    }),
                    arguments: vec![*inner.typ.clone()],
                });
            }
            _ => {}
        }
    }
}
