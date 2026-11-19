use std::{cell::RefCell, rc::Rc};

use gosyn::token::Operator;

use crate::{
    ast::{Expression, Ident, NameType, Operation, Type},
    name_resolution::Context,
};

pub fn infer_type(expr: &Expression, context: &Context) -> Type {
    match expr {
        Expression::Star(expr) => infer_type(&*expr.right, context),
        Expression::Call(call) => todo!(),
        Expression::Index(index) => todo!(),
        Expression::IndexList(index_list) => todo!(),
        Expression::Slice(slice) => todo!(),
        Expression::Ident(ident) => resolve_type(ident, context),
        Expression::FuncLit(func_lit) => Type::Function(func_lit.typ.clone()),
        Expression::Ellipsis(ellipsis) => todo!(),
        Expression::Selector(selector) => todo!(),
        Expression::BasicLit(basic_lit) => todo!(),
        Expression::Range(range_expr) => todo!(),
        Expression::Paren(expr) => infer_type(&*expr.expr, context),
        Expression::TypeAssert(type_assertion) => todo!(),
        Expression::CompositeLit(composite_lit) => todo!(),
        Expression::List(expressions) => todo!(),
        Expression::Operation(op) => infer_type_for_operation(op, context),
        Expression::Type(typ) => typ.clone(),
    }
}

pub fn infer_type_for_operation(op: &Operation, context: &Context) -> Type {
    match op.op {
        Operator::NotEqual
        | Operator::Less
        | Operator::LessEqual
        | Operator::Greater
        | Operator::GreaterEqual
        | Operator::AndAnd
        | Operator::OrOr
        | Operator::Equal
        | Operator::Not => Type::Name(NameType::Ident(Ident {
            name: "bool".to_string(),
        })),
        _ => unimplemented![],
    }
}

fn resolve_type(ident: &Ident, context: &Context) -> Type {
    todo![]
}
