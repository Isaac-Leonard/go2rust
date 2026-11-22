use std::fmt::Debug;

use crate::utils::VecExt;

use super::{BlockStmt, Comment, Expression, FuncType, Ident, ParameterSpec, Type, TypeParamDecl};

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
