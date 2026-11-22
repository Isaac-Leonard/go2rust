use gosyn::token::{Keyword, Operator};

use crate::utils::VecExt;

use super::{Call, ConstSpec, Decl, Expression, Ident, ShortVarDecl, Type, TypeSpec, VarSpec};

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
            op: value.op,
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
            op: value.op,
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
    pub op: Option<Operator>, // define or assign
    pub expr: Expression,
    pub body: BlockStmt,
}

impl From<gosyn::ast::RangeStmt> for RangeStmt {
    fn from(value: gosyn::ast::RangeStmt) -> Self {
        Self {
            key: value.key.map(Into::into),
            value: value.value.map(Into::into),
            op: value.op.map(|(_, op)| op),
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
            key: value.key,
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
