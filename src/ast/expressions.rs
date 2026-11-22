use gosyn::token::{LitKind, Operator};

use std::fmt::Debug;

use crate::utils::VecExt;

use super::{BlockStmt, FuncType, Ident, Type};

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
