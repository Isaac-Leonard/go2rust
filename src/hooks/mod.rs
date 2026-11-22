use crate::{
    ast::{Expression, Ident, InstantiatedType, NameType, PointerType, Type},
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
