use gosyn::token::{Keyword, Operator};

use crate::{
    ast::{
        AssignStmt, BlockStmt, BranchStmt, ExprSwitchStmt, Expression, FuncType, Ident, IfStmt,
        InstantiatedType, NameType, Operation, ShortVarDecl, Statement, SwitchStmt, Type,
    },
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

fn has_fallthrough(switch: &ExprSwitchStmt) -> bool {
    for clause in &switch.block.body {
        for stmt in &clause.body {
            if let Statement::Branch(branch) = stmt
                && branch.key == Keyword::FallThrough
            {
                return true;
            }
            if stmt_has_fallthrough(stmt) {
                return true;
            }
        }
    }
    false
}

fn stmt_has_fallthrough(stmt: &Statement) -> bool {
    match stmt {
        Statement::If(if_stmt) => {
            if block_stmt_has_fallthrough(&if_stmt.body) {
                return true;
            }
            if let Some(else_) = if_stmt.else_.as_deref()
                && stmt_has_fallthrough(else_)
            {
                return true;
            }
        }
        Statement::For(for_stmt) => {
            if block_stmt_has_fallthrough(&for_stmt.body) {
                return true;
            }
        }
        Statement::Block(block_stmt) => {
            if block_stmt_has_fallthrough(block_stmt) {
                return true;
            }
        }
        Statement::Range(range_stmt) => {
            if block_stmt_has_fallthrough(&range_stmt.body) {
                return true;
            }
        }
        Statement::Label(labeled_stmt) => {
            if stmt_has_fallthrough(&labeled_stmt.stmt) {
                return true;
            }
        }
        Statement::Select(select_stmt) => {
            for clause in &select_stmt.body.body {
                for stmt in &clause.body {
                    if stmt_has_fallthrough(stmt) {
                        return true;
                    }
                }
            }
        }
        Statement::Declaration(_)
        | Statement::ShortVarDecl(_)
        | Statement::Go(_)
        | Statement::Send(_)
        | Statement::Expr(_)
        | Statement::Defer(_)
        | Statement::Empty(_)
        | Statement::IncDec(_)
        | Statement::Return(_)
        | Statement::Branch(_)
        | Statement::Assign(_)
        | Statement::Switch(_) => {
            // None of these contain relevant statements so we have nothing to do.
        }
    }
    false
}

fn block_stmt_has_fallthrough(block_stmt: &crate::ast::BlockStmt) -> bool {
    for stmt in &block_stmt.list {
        if stmt_has_fallthrough(stmt) {
            return true;
        }
    }
    false
}

pub fn transform_switches_with_fallthrough_to_if(stmt: &mut Statement, _context: &mut Context) {
    let Statement::Switch(SwitchStmt::Expr(switch)) = stmt else {
        return;
    };
    if !has_fallthrough(switch) {
        return;
    }

    let true_expr = Expression::Ident(Ident {
        name: "true".to_string(),
    });
    let false_expr = Expression::Ident(Ident {
        name: "false".to_string(),
    });

    let fallthrough = Ident {
        name: "fallthrough".to_string(),
    };
    let fallthrough_decl = ShortVarDecl {
        name: vec![fallthrough.clone()],
        values: vec![false_expr.clone()],
    };
    let fallthrough = Expression::Ident(fallthrough);

    let init = switch.init.clone();

    let tag = Ident {
        name: "tag".to_string(),
    };
    let tag_decl = ShortVarDecl {
        name: vec![tag.clone()],
        values: vec![switch.tag.clone().unwrap_or_else(|| true_expr.clone())],
    };
    let tag = Expression::Ident(tag);

    let if_stmts = switch.block.body.iter().map(|clause| {
        let mut cond = fallthrough.clone();
        for case in &clause.list {
            let case_cond = Expression::Operation(Operation {
                op: Operator::Equal,
                x: Box::new(case.clone()),
                y: Some(Box::new(tag.clone())),
            });
            cond = Expression::Operation(Operation {
                op: Operator::OrOr,
                x: Box::new(cond),
                y: Some(Box::new(case_cond)),
            });
        }
        let mut body = clause
            .body
            .iter()
            .map(|stmt| {
                let mut stmt = stmt.clone();
                replace_fallthrough_in_statement_in_switch_clause(&mut stmt);
                stmt
            })
            .collect::<Vec<_>>();
        body.insert(
            0,
            Statement::Assign(AssignStmt {
                op: Operator::Assign,
                left: vec![fallthrough.clone()],
                right: vec![false_expr.clone()],
            }),
        );
        let body = BlockStmt { list: body };
        Statement::If(IfStmt {
            init: None,
            cond,
            body,
            else_: None,
        })
    });
    let mut stmts = vec![
        Statement::ShortVarDecl(fallthrough_decl),
        Statement::ShortVarDecl(tag_decl),
    ];
    if let Some(init) = init {
        stmts.push(*init);
    }
    stmts.extend(if_stmts);
    *stmt = Statement::Block(BlockStmt { list: stmts })
}

fn replace_fallthrough_in_statement_in_switch_clause(stmt: &mut Statement) {
    match stmt {
        Statement::Branch(BranchStmt {
            key: Keyword::FallThrough,
            ident: _,
        }) => {
            *stmt = Statement::Assign(AssignStmt {
                op: Operator::Assign,
                left: vec![Expression::Ident(Ident {
                    name: "fallthrough".to_string(),
                })],
                right: vec![Expression::Ident(Ident {
                    name: "true".to_string(),
                })],
            })
        }
        Statement::If(if_stmt) => {
            replace_fallthrough_in_block_statement_in_switch_clause(&mut if_stmt.body);
            if let Some(else_) = if_stmt.else_.as_deref_mut() {
                replace_fallthrough_in_statement_in_switch_clause(else_);
            }
        }
        Statement::For(for_stmt) => {
            replace_fallthrough_in_block_statement_in_switch_clause(&mut for_stmt.body)
        }
        Statement::Block(block_stmt) => {
            replace_fallthrough_in_block_statement_in_switch_clause(block_stmt)
        }
        Statement::Range(range_stmt) => {
            replace_fallthrough_in_block_statement_in_switch_clause(&mut range_stmt.body)
        }
        Statement::Label(labeled_stmt) => {
            replace_fallthrough_in_statement_in_switch_clause(&mut labeled_stmt.stmt);
        }
        Statement::Select(select_stmt) => {
            for clause in &mut select_stmt.body.body {
                for stmt in &mut clause.body {
                    replace_fallthrough_in_statement_in_switch_clause(stmt);
                }
            }
        }
        Statement::Declaration(_)
        | Statement::ShortVarDecl(_)
        | Statement::Go(_)
        | Statement::Send(_)
        | Statement::Expr(_)
        | Statement::Defer(_)
        | Statement::Empty(_)
        | Statement::IncDec(_)
        | Statement::Return(_)
        | Statement::Branch(_)
        | Statement::Assign(_)
        | Statement::Switch(_) => {
            // None of these contain relevant statements so we have nothing to do.
        }
    }
}

fn replace_fallthrough_in_block_statement_in_switch_clause(block_stmt: &mut BlockStmt) {
    for stmt in &mut block_stmt.list {
        replace_fallthrough_in_statement_in_switch_clause(stmt);
    }
}
