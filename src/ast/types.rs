use super::{Expression, Ident, QualifiedName};

#[derive(Debug, Clone)]
pub struct PointerType {
    pub typ: Box<Type>,
}

impl From<gosyn::ast::PointerType> for PointerType {
    fn from(value: gosyn::ast::PointerType) -> Self {
        Self {
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub len: Box<Expression>,
    pub typ: Box<Type>,
}

impl From<gosyn::ast::ArrayType> for ArrayType {
    fn from(value: gosyn::ast::ArrayType) -> Self {
        Self {
            len: value.len.into(),
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SliceType {
    pub typ: Box<Type>,
}

impl From<gosyn::ast::SliceType> for SliceType {
    fn from(value: gosyn::ast::SliceType) -> Self {
        Self {
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MapType {
    pub key: Box<Type>,
    pub val: Box<Type>,
}

impl From<gosyn::ast::MapType> for MapType {
    fn from(value: gosyn::ast::MapType) -> Self {
        Self {
            key: value.key.into(),
            val: value.val.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub field: Field,
    pub tag: Option<gosyn::ast::StringLit>,
}

impl From<gosyn::ast::FieldDecl> for FieldDecl {
    fn from(value: gosyn::ast::FieldDecl) -> Self {
        Self {
            field: value.field.into(),
            tag: value.tag,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Field {
    Named(NamedField),
    Embedded(EmbeddedField),
}

impl From<gosyn::ast::Field> for Field {
    fn from(value: gosyn::ast::Field) -> Self {
        match value {
            gosyn::ast::Field::Named(field) => Self::Named(field.into()),
            gosyn::ast::Field::Embedded(field) => Self::Embedded(field.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NamedField {
    pub name: Vec<Ident>,
    pub typ: Type,
}

impl From<gosyn::ast::NamedField> for NamedField {
    fn from(value: gosyn::ast::NamedField) -> Self {
        Self {
            name: value.name.into_iter().map(Into::into).collect(),
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedField {
    pub pointer: bool,
    pub typ: NameType,
    pub type_args: Vec<Type>,
}

impl From<gosyn::ast::EmbeddedField> for EmbeddedField {
    fn from(value: gosyn::ast::EmbeddedField) -> Self {
        Self {
            pointer: value.pointer.is_some(),
            typ: value.typ.into(),
            type_args: value.type_args.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodElem {
    pub name: Ident,
    pub typ: FuncType,
}

impl From<gosyn::ast::MethodElem> for MethodElem {
    fn from(value: gosyn::ast::MethodElem) -> Self {
        Self {
            name: value.name.into(),
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructType {
    pub fields: Vec<FieldDecl>,
}

impl From<gosyn::ast::StructType> for StructType {
    fn from(value: gosyn::ast::StructType) -> Self {
        StructType {
            fields: value.fields.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParameterSpec {
    pub identifiers: Vec<Ident>,
    pub variadic: bool, // If Some then represents the position of the ... operator
    pub typ: Type,
}

impl From<gosyn::ast::ParameterSpec> for ParameterSpec {
    fn from(value: gosyn::ast::ParameterSpec) -> Self {
        Self {
            identifiers: value.identifiers.into_iter().map(Into::into).collect(),
            variadic: value.variadic.is_some(),
            typ: value.typ.into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct FuncType {
    pub typ_params: Vec<TypeParamDecl>,
    pub params: Vec<ParameterSpec>,
    pub result: Vec<ParameterSpec>,
}

impl From<gosyn::ast::FuncType> for FuncType {
    fn from(value: gosyn::ast::FuncType) -> Self {
        Self {
            typ_params: value.typ_params.into_iter().map(Into::into).collect(),
            params: value
                .params
                .parameters
                .into_iter()
                .map(Into::into)
                .collect(),
            result: value
                .result
                .parameters
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeParamDecl {
    pub name: Vec<Ident>,
    pub constraint: TypeElem,
}

impl From<gosyn::ast::TypeParamDecl> for TypeParamDecl {
    fn from(value: gosyn::ast::TypeParamDecl) -> Self {
        Self {
            name: value.name.into_iter().map(Into::into).collect(),
            constraint: value.constraint.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelType {
    pub dir: Option<gosyn::ast::ChanMode>,
    pub typ: Box<Type>,
}

impl From<gosyn::ast::ChannelType> for ChannelType {
    fn from(value: gosyn::ast::ChannelType) -> Self {
        Self {
            dir: value.dir.into(),
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceElem {
    Method(MethodElem),
    Type(TypeElem),
}

impl From<gosyn::ast::InterfaceElem> for InterfaceElem {
    fn from(value: gosyn::ast::InterfaceElem) -> Self {
        match value {
            gosyn::ast::InterfaceElem::Method(method) => Self::Method(method.into()),
            gosyn::ast::InterfaceElem::Type(typ) => Self::Type(typ.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceType {
    pub elements: Vec<InterfaceElem>,
}

impl From<gosyn::ast::InterfaceType> for InterfaceType {
    fn from(value: gosyn::ast::InterfaceType) -> Self {
        Self {
            elements: value.elements.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NameType {
    Ident(Ident),
    Qualified(QualifiedName),
}

impl From<gosyn::ast::NameType> for NameType {
    fn from(value: gosyn::ast::NameType) -> Self {
        match value {
            gosyn::ast::NameType::Ident(ident) => NameType::Ident(ident.into()),
            gosyn::ast::NameType::Qualified(name) => NameType::Qualified(name.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstantiatedType {
    pub name: NameType,
    pub arguments: Vec<Type>,
}

impl From<gosyn::ast::InstantiatedType> for InstantiatedType {
    fn from(value: gosyn::ast::InstantiatedType) -> Self {
        Self {
            name: value.name.into(),
            arguments: value.arguments.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParenType {
    pub typ: Box<Type>,
}

impl From<gosyn::ast::ParenType> for ParenType {
    fn from(value: gosyn::ast::ParenType) -> Self {
        Self {
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnderLyingType {
    pub typ: Box<Type>,
}

impl From<gosyn::ast::UnderLyingType> for UnderLyingType {
    fn from(value: gosyn::ast::UnderLyingType) -> Self {
        Self {
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeTerm {
    Type(Type),
    UnderLyingType(UnderLyingType),
}

impl From<gosyn::ast::TypeTerm> for TypeTerm {
    fn from(value: gosyn::ast::TypeTerm) -> Self {
        match value {
            gosyn::ast::TypeTerm::Type(typ) => TypeTerm::Type(typ.into()),
            gosyn::ast::TypeTerm::UnderLyingType(typ) => Self::UnderLyingType(typ.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeElem {
    pub types: Vec<TypeTerm>,
}

impl From<gosyn::ast::TypeElem> for TypeElem {
    fn from(value: gosyn::ast::TypeElem) -> Self {
        Self {
            types: value.types.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Type {
    Name(NameType),
    Instantiated(InstantiatedType),
    Map(MapType),             // map[K]V
    Array(ArrayType),         // [N]T
    Slice(SliceType),         // []T
    Function(FuncType),       // func (...) ...
    Struct(StructType),       // struct { ... }
    Channel(ChannelType),     // <-chan T | chan<- T | chan T
    Pointer(PointerType),     // *T
    Interface(InterfaceType), // interface { ... }
    Paren(ParenType),
}

impl From<gosyn::ast::Type> for Type {
    fn from(value: gosyn::ast::Type) -> Self {
        match value {
            gosyn::ast::Type::Name(name) => Type::Name(name.into()),
            gosyn::ast::Type::Instantiated(instantiated) => Type::Instantiated(instantiated.into()),
            gosyn::ast::Type::Map(map) => Type::Map(map.into()),
            gosyn::ast::Type::Array(array) => Type::Array(array.into()),
            gosyn::ast::Type::Slice(slice) => Type::Slice(slice.into()),
            gosyn::ast::Type::Function(func) => Type::Function(func.into()),
            gosyn::ast::Type::Struct(struct_type) => Self::Struct(struct_type.into()),
            gosyn::ast::Type::Channel(channel) => Type::Channel(channel.into()),
            gosyn::ast::Type::Pointer(pointer) => Type::Pointer(pointer.into()),
            gosyn::ast::Type::Interface(interface) => Type::Interface(interface.into()),
            gosyn::ast::Type::Paren(paren) => Self::Paren(paren.into()),
        }
    }
}

impl From<Box<gosyn::ast::Type>> for Box<Type> {
    fn from(value: Box<gosyn::ast::Type>) -> Self {
        Box::new((*value).into())
    }
}
