//! Golang AST nodes without positional information

use gosyn::token::Keyword;
use gosyn::token::LitKind;
use gosyn::token::Operator;

use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;

use crate::utils::VecExt;

#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,
}

impl From<gosyn::ast::Comment> for Comment {
    fn from(value: gosyn::ast::Comment) -> Self {
        Self { text: value.text }
    }
}

impl From<Rc<gosyn::ast::Comment>> for Comment {
    fn from(value: Rc<gosyn::ast::Comment>) -> Self {
        Self {
            text: value.text.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Ident {
    pub name: String,
}

impl From<gosyn::ast::Ident> for Ident {
    fn from(value: gosyn::ast::Ident) -> Self {
        Self {
            name: value.name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QualifiedName {
    pub package: Ident,
    pub identifier: Ident,
}

impl From<gosyn::ast::QualifiedName> for QualifiedName {
    fn from(value: gosyn::ast::QualifiedName) -> Self {
        Self {
            package: value.package.into(),
            identifier: value.identifier.into(),
        }
    }
}

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
    pub pointer: Option<usize>,
    pub typ: NameType,
    pub type_args: Vec<Type>,
}

impl From<gosyn::ast::EmbeddedField> for EmbeddedField {
    fn from(value: gosyn::ast::EmbeddedField) -> Self {
        Self {
            pointer: value.pointer,
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
    pub variadic: Option<usize>, // If Some then represents the position of the ... operator
    pub typ: Type,
}

impl From<gosyn::ast::ParameterSpec> for ParameterSpec {
    fn from(value: gosyn::ast::ParameterSpec) -> Self {
        Self {
            identifiers: value.identifiers.into_iter().map(Into::into).collect(),
            variadic: value.variadic,
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

// ================ Expression Definition ================

#[derive(Debug, Clone)]
pub struct BasicLit {
    pub kind: LitKind,
    pub value: String,
}

impl From<gosyn::ast::BasicLit> for BasicLit {
    fn from(value: gosyn::ast::BasicLit) -> Self {
        Self {
            kind: value.kind,
            value: value.value,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct StringLit {
    pub value: String,
}

impl From<gosyn::ast::StringLit> for StringLit {
    fn from(value: gosyn::ast::StringLit) -> Self {
        Self { value: value.value }
    }
}

#[derive(Debug, Clone)]
pub struct FuncLit {
    pub typ: FuncType,
    pub body: BlockStmt,
}

impl From<gosyn::ast::FuncLit> for FuncLit {
    fn from(value: gosyn::ast::FuncLit) -> Self {
        Self {
            typ: value.typ.into(),
            body: value.body.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Element {
    Expr(Expression),
    LitValue(LiteralValue),
}

impl From<gosyn::ast::Element> for Element {
    fn from(value: gosyn::ast::Element) -> Self {
        match value {
            gosyn::ast::Element::Expr(expression) => Self::Expr(expression.into()),
            gosyn::ast::Element::LitValue(literal_value) => Self::LitValue(literal_value.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyedElement {
    pub key: Option<Element>,
    pub val: Element,
}

impl From<gosyn::ast::KeyedElement> for KeyedElement {
    fn from(value: gosyn::ast::KeyedElement) -> Self {
        Self {
            key: value.key.map(Into::into),
            val: value.val.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiteralValue {
    pub values: Vec<KeyedElement>,
}

impl From<gosyn::ast::LiteralValue> for LiteralValue {
    fn from(value: gosyn::ast::LiteralValue) -> Self {
        Self {
            values: value.values.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompositeLit {
    pub typ: Box<Expression>,
    pub val: LiteralValue,
}

impl From<gosyn::ast::CompositeLit> for CompositeLit {
    fn from(value: gosyn::ast::CompositeLit) -> Self {
        Self {
            typ: value.typ.into(),
            val: value.val.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Selector {
    pub x: Box<Expression>,
    pub sel: Ident,
}

impl From<gosyn::ast::Selector> for Selector {
    fn from(value: gosyn::ast::Selector) -> Self {
        Self {
            x: value.x.into(),
            sel: value.sel.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeAssertion {
    pub left: Box<Expression>,
    pub right: Option<Box<Type>>, // None for x.(type)
}

impl From<gosyn::ast::TypeAssertion> for TypeAssertion {
    fn from(value: gosyn::ast::TypeAssertion) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Index {
    pub left: Box<Expression>,
    pub index: Box<Expression>,
}

impl From<gosyn::ast::Index> for Index {
    fn from(value: gosyn::ast::Index) -> Self {
        Self {
            left: value.left.into(),
            index: value.index.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexList {
    pub left: Box<Expression>,
    pub indices: Vec<Expression>,
}

impl From<gosyn::ast::IndexList> for IndexList {
    fn from(value: gosyn::ast::IndexList) -> Self {
        Self {
            left: value.left.into(),
            indices: value.indices.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Slice {
    pub left: Box<Expression>,
    pub index: [Option<Box<Expression>>; 3],
}

impl From<gosyn::ast::Slice> for Slice {
    fn from(value: gosyn::ast::Slice) -> Self {
        Self {
            left: value.left.into(),
            index: value.index.map(|i| i.map(Into::into)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Call {
    pub args: Vec<Expression>,
    pub func: Box<Expression>,
    pub dots: bool,
}

impl From<gosyn::ast::Call> for Call {
    fn from(value: gosyn::ast::Call) -> Self {
        Self {
            args: value.args.map_into(),
            func: value.func.into(),
            dots: value.dots.is_some(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParenExpression {
    pub expr: Box<Expression>,
}

impl From<gosyn::ast::ParenExpression> for ParenExpression {
    fn from(value: gosyn::ast::ParenExpression) -> Self {
        Self {
            expr: value.expr.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StarExpression {
    pub right: Box<Expression>,
}

impl From<gosyn::ast::StarExpression> for StarExpression {
    fn from(value: gosyn::ast::StarExpression) -> Self {
        Self {
            right: value.right.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ellipsis {
    pub elt: Option<Box<Expression>>,
}

impl From<gosyn::ast::Ellipsis> for Ellipsis {
    fn from(value: gosyn::ast::Ellipsis) -> Self {
        Self {
            elt: value.elt.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RangeExpr {
    pub right: Box<Expression>,
}

impl From<gosyn::ast::RangeExpr> for RangeExpr {
    fn from(value: gosyn::ast::RangeExpr) -> Self {
        Self {
            right: value.right.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub op: Operator,
    pub x: Box<Expression>,
    pub y: Option<Box<Expression>>,
}

impl From<gosyn::ast::Operation> for Operation {
    fn from(value: gosyn::ast::Operation) -> Self {
        Self {
            op: value.op,
            x: value.x.into(),
            y: value.y.map(Into::into),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Expression {
    Call(Call),
    Index(Index),
    IndexList(IndexList),
    Slice(Slice),
    Ident(Ident),
    FuncLit(FuncLit),
    Ellipsis(Ellipsis),
    Selector(Selector),
    BasicLit(BasicLit),
    Range(RangeExpr),
    Star(StarExpression),
    Paren(ParenExpression),
    TypeAssert(TypeAssertion),
    CompositeLit(CompositeLit),
    List(Vec<Expression>),
    Operation(Operation),
    Type(Type),
}

impl From<gosyn::ast::Expression> for Expression {
    fn from(value: gosyn::ast::Expression) -> Self {
        match value {
            gosyn::ast::Expression::Call(call) => Self::Call(call.into()),
            gosyn::ast::Expression::Index(index) => Self::Index(index.into()),
            gosyn::ast::Expression::IndexList(index_list) => Self::IndexList(index_list.into()),
            gosyn::ast::Expression::Slice(slice) => Self::Slice(slice.into()),
            gosyn::ast::Expression::Ident(ident) => Self::Ident(ident.into()),
            gosyn::ast::Expression::FuncLit(func_lit) => Self::FuncLit(func_lit.into()),
            gosyn::ast::Expression::Ellipsis(ellipsis) => Self::Ellipsis(ellipsis.into()),
            gosyn::ast::Expression::Selector(selector) => Self::Selector(selector.into()),
            gosyn::ast::Expression::BasicLit(basic_lit) => Self::BasicLit(basic_lit.into()),
            gosyn::ast::Expression::Range(range_expr) => Self::Range(range_expr.into()),
            gosyn::ast::Expression::Star(star_expression) => Self::Star(star_expression.into()),
            gosyn::ast::Expression::Paren(paren_expression) => Self::Paren(paren_expression.into()),
            gosyn::ast::Expression::TypeAssert(type_assertion) => {
                Self::TypeAssert(type_assertion.into())
            }
            gosyn::ast::Expression::CompositeLit(composite_lit) => {
                Self::CompositeLit(composite_lit.into())
            }
            gosyn::ast::Expression::List(expressions) => Self::List(expressions.map_into()),
            gosyn::ast::Expression::Operation(operation) => Self::Operation(operation.into()),
            gosyn::ast::Expression::Type(type_) => Self::Type(type_.into()),
        }
    }
}

impl From<Box<gosyn::ast::Expression>> for Box<Expression> {
    fn from(value: Box<gosyn::ast::Expression>) -> Self {
        Box::new((*value).into())
    }
}

// ================ Declaration Definition ================

#[derive(Debug, Clone)]
pub struct Decl<T> {
    pub docs: Vec<Comment>,
    pub specs: Vec<T>,
}

impl<T, R> From<gosyn::ast::Decl<T>> for Decl<R>
where
    R: From<T>,
{
    fn from(value: gosyn::ast::Decl<T>) -> Self {
        Self {
            docs: value.docs.map_into(),
            specs: value.specs.map_into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct VarSpec {
    pub docs: Vec<Comment>,
    pub name: Vec<Ident>,
    pub typ: Option<Type>,
    pub values: Vec<Expression>,
}

impl From<gosyn::ast::VarSpec> for VarSpec {
    fn from(value: gosyn::ast::VarSpec) -> Self {
        Self {
            docs: value.docs.map_into(),
            name: value.name.map_into(),
            typ: value.typ.map(Into::into),
            values: value.values.map_into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ShortVarDecl {
    pub name: Vec<Ident>,
    pub values: Vec<Expression>,
}

impl From<gosyn::ast::ShortVarDecl> for ShortVarDecl {
    fn from(value: gosyn::ast::ShortVarDecl) -> Self {
        Self {
            name: value.name.map_into(),
            values: value.values.map_into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ConstSpec {
    pub docs: Vec<Comment>,
    pub name: Vec<Ident>,
    pub typ: Option<Type>,
    pub values: Vec<Expression>,
}

impl From<gosyn::ast::ConstSpec> for ConstSpec {
    fn from(value: gosyn::ast::ConstSpec) -> Self {
        Self {
            docs: value.docs.map_into(),
            name: value.name.map_into(),
            typ: value.typ.map(Into::into),
            values: value.values.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeSpec {
    pub docs: Vec<Comment>,
    pub alias: bool,
    pub name: Ident,
    pub params: Vec<TypeParamDecl>,
    pub typ: Type,
}

impl From<gosyn::ast::TypeSpec> for TypeSpec {
    fn from(value: gosyn::ast::TypeSpec) -> Self {
        Self {
            docs: value.docs.map_into(),
            alias: value.alias,
            name: value.name.into(),
            params: value.params.map_into(),
            typ: value.typ.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub docs: Vec<Comment>,
    pub recv: Option<Vec<ParameterSpec>>,
    pub name: Ident,
    pub typ: FuncType,
    pub body: Option<BlockStmt>,
}

impl From<gosyn::ast::FuncDecl> for FuncDecl {
    fn from(value: gosyn::ast::FuncDecl) -> Self {
        Self {
            docs: value.docs.map_into(),
            recv: value.recv.map(|p| p.parameters.map_into()),
            name: value.name.into(),
            typ: value.typ.into(),
            body: value.body.map(Into::into),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Declaration {
    Function(FuncDecl),
    Type(Decl<TypeSpec>),
    Const(Decl<ConstSpec>),
    Variable(Decl<VarSpec>),
}

impl From<gosyn::ast::Declaration> for Declaration {
    fn from(value: gosyn::ast::Declaration) -> Self {
        match value {
            gosyn::ast::Declaration::Function(func_decl) => Self::Function(func_decl.into()),
            gosyn::ast::Declaration::Type(decl) => Self::Type(decl.into()),
            gosyn::ast::Declaration::Const(decl) => Self::Const(decl.into()),
            gosyn::ast::Declaration::Variable(decl) => Self::Variable(decl.into()),
        }
    }
}

// ================ Statement Definition ================

#[derive(Debug, Clone)]
pub struct BlockStmt {
    pub list: Vec<Statement>,
}

impl From<gosyn::ast::BlockStmt> for BlockStmt {
    fn from(value: gosyn::ast::BlockStmt) -> Self {
        Self {
            list: value.list.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeclStmt {
    Type(Decl<TypeSpec>),
    Const(Decl<ConstSpec>),
    Variable(Decl<VarSpec>),
}

impl From<gosyn::ast::DeclStmt> for DeclStmt {
    fn from(value: gosyn::ast::DeclStmt) -> Self {
        match value {
            gosyn::ast::DeclStmt::Type(decl) => DeclStmt::Type(decl.into()),
            gosyn::ast::DeclStmt::Const(decl) => Self::Const(decl.into()),
            gosyn::ast::DeclStmt::Variable(decl) => Self::Variable(decl.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoStmt {
    pub call: Call,
}

impl From<gosyn::ast::GoStmt> for GoStmt {
    fn from(value: gosyn::ast::GoStmt) -> Self {
        Self {
            call: value.call.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeferStmt {
    pub call: Call,
}

impl From<gosyn::ast::DeferStmt> for DeferStmt {
    fn from(value: gosyn::ast::DeferStmt) -> Self {
        Self {
            call: value.call.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub ret: Vec<Expression>,
}

impl From<gosyn::ast::ReturnStmt> for ReturnStmt {
    fn from(value: gosyn::ast::ReturnStmt) -> Self {
        Self {
            ret: value.ret.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BranchStmt {
    pub key: Keyword,
    pub ident: Option<Ident>,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub init: Option<Box<Statement>>,
    pub cond: Expression,
    pub body: BlockStmt,
    pub else_: Option<Box<Statement>>,
}

impl From<gosyn::ast::IfStmt> for IfStmt {
    fn from(value: gosyn::ast::IfStmt) -> Self {
        Self {
            init: value.init.map(Into::into),
            cond: value.cond.into(),
            body: value.body.into(),
            else_: value.else_.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    // position of assign operator like = | += | &=
    pub op: Operator,
    pub left: Vec<Expression>,
    pub right: Vec<Expression>,
}

impl From<gosyn::ast::AssignStmt> for AssignStmt {
    fn from(value: gosyn::ast::AssignStmt) -> Self {
        Self {
            op: value.op.into(),
            left: value.left.map_into(),
            right: value.right.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LabeledStmt {
    pub name: Ident,
    pub stmt: Box<Statement>,
}

impl From<gosyn::ast::LabeledStmt> for LabeledStmt {
    fn from(value: gosyn::ast::LabeledStmt) -> Self {
        Self {
            name: value.name.into(),
            stmt: value.stmt.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendStmt {
    pub chan: Expression,
    pub value: Expression,
}

impl From<gosyn::ast::SendStmt> for SendStmt {
    fn from(value: gosyn::ast::SendStmt) -> Self {
        Self {
            chan: value.chan.into(),
            value: value.value.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expression,
}

impl From<gosyn::ast::ExprStmt> for ExprStmt {
    fn from(value: gosyn::ast::ExprStmt) -> Self {
        Self {
            expr: value.expr.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExprCaseClause {
    pub tok: Keyword,

    pub list: Vec<Expression>,
    pub body: Vec<Statement>,
}

impl From<gosyn::ast::ExprCaseClause> for ExprCaseClause {
    fn from(value: gosyn::ast::ExprCaseClause) -> Self {
        Self {
            tok: value.tok,
            list: value.list.map_into(),
            body: (*value.body).map_into(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct TypeCaseClause {
    pub tok: Keyword,

    pub list: Vec<Type>,
    pub body: Vec<Statement>,
}

impl From<gosyn::ast::TypeCaseClause> for TypeCaseClause {
    fn from(value: gosyn::ast::TypeCaseClause) -> Self {
        Self {
            tok: value.tok,
            list: value.list.map_into(),
            body: (*value.body).map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CaseBlock<T> {
    pub body: Vec<T>,
}

impl<T, R> From<gosyn::ast::CaseBlock<T>> for CaseBlock<R>
where
    R: From<T>,
{
    fn from(value: gosyn::ast::CaseBlock<T>) -> Self {
        Self {
            body: value.body.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExprSwitchStmt {
    pub init: Option<Box<Statement>>,
    pub tag: Option<Expression>,
    pub block: CaseBlock<ExprCaseClause>,
}

impl From<gosyn::ast::ExprSwitchStmt> for ExprSwitchStmt {
    fn from(value: gosyn::ast::ExprSwitchStmt) -> Self {
        Self {
            init: value.init.map(Into::into),
            tag: value.tag.map(Into::into),
            block: value.block.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeSwitchStmt {
    pub init: Option<Box<Statement>>,
    pub tag: Option<Box<Statement>>,
    pub block: CaseBlock<TypeCaseClause>,
}

impl From<gosyn::ast::TypeSwitchStmt> for TypeSwitchStmt {
    fn from(value: gosyn::ast::TypeSwitchStmt) -> Self {
        Self {
            init: value.init.map(Into::into),
            tag: value.tag.map(Into::into),
            block: value.block.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SwitchStmt {
    Expr(ExprSwitchStmt),
    Type(TypeSwitchStmt),
}

impl From<gosyn::ast::SwitchStmt> for SwitchStmt {
    fn from(value: gosyn::ast::SwitchStmt) -> Self {
        match value {
            gosyn::ast::SwitchStmt::Expr(expr_switch_stmt) => {
                SwitchStmt::Expr(expr_switch_stmt.into())
            }
            gosyn::ast::SwitchStmt::Type(type_switch_stmt) => Self::Type(type_switch_stmt.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IncDecStmt {
    pub op: Operator,
    pub expr: Expression,
}

impl From<gosyn::ast::IncDecStmt> for IncDecStmt {
    fn from(value: gosyn::ast::IncDecStmt) -> Self {
        Self {
            op: value.op.into(),
            expr: value.expr.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommClause {
    pub tok: Keyword,
    pub comm: Option<Box<Statement>>,
    pub body: Vec<Statement>,
}

impl From<gosyn::ast::CommClause> for CommClause {
    fn from(value: gosyn::ast::CommClause) -> Self {
        Self {
            tok: value.tok,
            comm: value.comm.map(Into::into),
            body: (*value.body).map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommBlock {
    pub body: Vec<CommClause>,
}

impl From<gosyn::ast::CommBlock> for CommBlock {
    fn from(value: gosyn::ast::CommBlock) -> Self {
        Self {
            body: value.body.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectStmt {
    pub body: CommBlock,
}

impl From<gosyn::ast::SelectStmt> for SelectStmt {
    fn from(value: gosyn::ast::SelectStmt) -> Self {
        Self {
            body: value.body.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RangeStmt {
    pub key: Option<Expression>,
    pub value: Option<Expression>,
    pub op: Option<(usize, Operator)>, // define or assign
    pub expr: Expression,
    pub body: BlockStmt,
}

impl From<gosyn::ast::RangeStmt> for RangeStmt {
    fn from(value: gosyn::ast::RangeStmt) -> Self {
        Self {
            key: value.key.map(Into::into),
            value: value.value.map(Into::into),
            op: value.op.into(),
            expr: value.expr.into(),
            body: value.body.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<Statement>>,
    pub cond: Option<Box<Statement>>,
    pub post: Option<Box<Statement>>,
    pub body: BlockStmt,
}

impl From<gosyn::ast::ForStmt> for ForStmt {
    fn from(value: gosyn::ast::ForStmt) -> Self {
        Self {
            init: value.init.map(Into::into),
            cond: value.cond.map(Into::into),
            post: value.post.map(Into::into),
            body: value.body.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmptyStmt;

impl From<gosyn::ast::EmptyStmt> for EmptyStmt {
    fn from(_value: gosyn::ast::EmptyStmt) -> Self {
        Self {}
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum Statement {
    Go(GoStmt),
    If(IfStmt),
    For(ForStmt),
    Send(SendStmt),
    Expr(ExprStmt),
    Defer(DeferStmt),
    Block(BlockStmt),
    Range(RangeStmt),
    Empty(EmptyStmt),
    Label(LabeledStmt),
    IncDec(IncDecStmt),
    Assign(AssignStmt),
    Return(ReturnStmt),
    Branch(BranchStmt),
    Switch(SwitchStmt),
    Select(SelectStmt),
    Declaration(DeclStmt),
    ShortVarDecl(ShortVarDecl),
}

impl From<gosyn::ast::BranchStmt> for BranchStmt {
    fn from(value: gosyn::ast::BranchStmt) -> Self {
        Self {
            key: value.key.into(),
            ident: value.ident.map(Into::into),
        }
    }
}

impl From<gosyn::ast::Statement> for Statement {
    fn from(value: gosyn::ast::Statement) -> Self {
        match value {
            gosyn::ast::Statement::Go(go_stmt) => Self::Go(go_stmt.into()),
            gosyn::ast::Statement::If(if_stmt) => Self::If(if_stmt.into()),
            gosyn::ast::Statement::For(for_stmt) => Self::For(for_stmt.into()),
            gosyn::ast::Statement::Send(send_stmt) => Self::Send(send_stmt.into()),
            gosyn::ast::Statement::Expr(expr_stmt) => Self::Expr(expr_stmt.into()),
            gosyn::ast::Statement::Defer(defer_stmt) => Self::Defer(defer_stmt.into()),
            gosyn::ast::Statement::Block(block_stmt) => Self::Block(block_stmt.into()),
            gosyn::ast::Statement::Range(range_stmt) => Self::Range(range_stmt.into()),
            gosyn::ast::Statement::Empty(empty_stmt) => Self::Empty(empty_stmt.into()),
            gosyn::ast::Statement::Label(labeled_stmt) => Self::Label(labeled_stmt.into()),
            gosyn::ast::Statement::IncDec(inc_dec_stmt) => Self::IncDec(inc_dec_stmt.into()),
            gosyn::ast::Statement::Assign(assign_stmt) => Self::Assign(assign_stmt.into()),
            gosyn::ast::Statement::Return(return_stmt) => Self::Return(return_stmt.into()),
            gosyn::ast::Statement::Branch(branch_stmt) => Self::Branch(branch_stmt.into()),
            gosyn::ast::Statement::Switch(switch_stmt) => Self::Switch(switch_stmt.into()),
            gosyn::ast::Statement::Select(select_stmt) => Self::Select(select_stmt.into()),
            gosyn::ast::Statement::Declaration(decl_stmt) => Self::Declaration(decl_stmt.into()),
            gosyn::ast::Statement::ShortVarDecl(short_var_decl) => {
                Self::ShortVarDecl(short_var_decl.into())
            }
        }
    }
}

impl From<Box<gosyn::ast::Statement>> for Box<Statement> {
    fn from(value: Box<gosyn::ast::Statement>) -> Self {
        Box::new((*value).into())
    }
}
#[derive(Default, Debug, Clone)]
pub struct Import {
    pub name: Option<Ident>,
    pub path: StringLit,
}

impl From<gosyn::ast::Import> for Import {
    fn from(value: gosyn::ast::Import) -> Self {
        Self {
            name: value.name.map(Into::into),
            path: value.path.into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub line_info: Vec<usize>,
    pub docs: Vec<Comment>,
    pub pkg_name: Ident,
    pub imports: Vec<Import>,
    pub decl: Vec<Declaration>,
    pub comments: Vec<Comment>,
}

impl From<gosyn::ast::File> for File {
    fn from(value: gosyn::ast::File) -> Self {
        Self {
            path: value.path,
            line_info: value.line_info,
            docs: value.docs.map_into(),
            pkg_name: value.pkg_name.into(),
            imports: value.imports.map_into(),
            decl: value.decl.map_into(),
            comments: value.comments.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub path: PathBuf,
    pub files: Vec<File>,
}

impl AssignStmt {
    pub fn is_range(&self) -> bool {
        matches!(self.right.first(), Some(Expression::Range(_)))
    }
}

impl ShortVarDecl {
    pub fn is_range(&self) -> bool {
        matches!(self.values.first(), Some(Expression::Range(_)))
    }
}
