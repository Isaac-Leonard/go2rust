use gosyn::token::Keyword;

use crate::{
    ast::{
        BlockStmt, BranchStmt, DeclStmt, ExprSwitchStmt, Expression, ForStmt, IfStmt, RangeStmt,
        ShortVarDecl, Statement, SwitchStmt,
    },
    name_resolution::Context,
    transform::expressions::convert_call,
};

use super::{
    convert_const_decl, convert_type_decl, convert_variable_decl, expressions::convert_expr,
};

/// Converts variable short form declarations to rust syntax.
/// This currently only outputs variables declared as let mut such as:
/// ```
/// let mut variable=value;
/// ```
/// This means that all variables are treated as mutable.
/// This function does not perform any type inference so will not output types on variables.
/// Multi variable declarations are split into multiple single variable declarations in rust.
///
fn convert_short_variable_decl(decl: &ShortVarDecl, context: &mut Context) -> String {
    let names = decl
        .name
        .iter()
        .map(|name| name.name.clone())
        .collect::<Vec<_>>();
    let values = decl
        .values
        .iter()
        .map(|value| convert_expr(value, context))
        .collect::<Vec<_>>();

    let names = match names.as_slice() {
        [name] => match name.as_str() {
            // Rust does not let us have mut on _ bindings
            "_" => name.to_string(),
            _ => format!("mut {name}"),
        },
        names => {
            let names = names
                .iter()
                .map(|name| match name.as_str() {
                    // Rust does not let us have mut on _ bindings
                    "_" => name.to_string(),
                    _ => format!("mut {name}"),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("({names})")
        }
    };

    let values = match values.as_slice() {
        [value] => value.to_string(),
        values => {
            let values = values.join(", ");
            format!("({values})")
        }
    };

    format!("let {names} = {values};")
}

pub fn convert_stmt(stmt: &Statement, context: &mut Context) -> String {
    match stmt {
        Statement::Expr(expr) => convert_expr(&expr.expr, context),
        Statement::Block(block) => convert_block_stmt(block, context),
        Statement::Return(ret) => match ret.ret.as_slice() {
            [] => "return;".to_string(),
            [expr] => {
                let expr = convert_expr(expr, context);
                format!("return {expr};")
            }
            exprs => {
                let exprs = exprs
                    .iter()
                    .map(|expr| convert_expr(expr, context))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("return ({exprs});")
            }
        },
        Statement::Assign(assign) => {
            // TODO We need to fix this to evaluate everything in the correct order.
            let left = assign
                .left
                .iter()
                .map(|expr| convert_expr(expr, context))
                .collect::<Vec<_>>();
            let right = assign
                .right
                .iter()
                .map(|expr| convert_expr(expr, context))
                .collect::<Vec<_>>();
            let op: &str = assign.op.into();
            left.into_iter()
                .zip(right)
                .map(|(left, right)| format!("{left} {op} {right}"))
                .collect::<Vec<_>>()
                .join(";\n")
        }
        Statement::Declaration(decl) => match decl {
            DeclStmt::Type(typ_decl) => convert_type_decl(typ_decl, context),
            DeclStmt::Const(const_decl) => convert_const_decl(const_decl, context),
            DeclStmt::Variable(var_decl) => convert_variable_decl(var_decl, context),
        },
        Statement::ShortVarDecl(decl) => convert_short_variable_decl(decl, context),
        Statement::If(if_stmt) => convert_if_stmt(if_stmt, context),
        Statement::For(for_stmt) => convert_for_stmt(for_stmt, context),
        Statement::Range(range_stmt) => convert_range_stmt(range_stmt, context),
        Statement::Empty(_) => "{}".to_string(),
        Statement::Branch(branch) => convert_branch_stmt(branch, context),
        Statement::Switch(SwitchStmt::Expr(expr_switch)) => {
            convert_expr_switch_stmt(expr_switch, context)
        }
        Statement::Defer(defer_stmt) => {
            let call = convert_call(&defer_stmt.call, context);
            format!("let _defer = core::mem::DropGuard::new((), || {call});")
        }
        other => unimplemented!["Cannot yet convert statement {other:?}"],
    }
}

pub fn convert_block_stmt(block: &BlockStmt, context: &mut Context) -> String {
    let statements = block
        .list
        .iter()
        .map(|stmt| convert_stmt(stmt, context))
        .collect::<Vec<_>>()
        .join(";\n");
    format!("{{\n{statements}\n}}")
}

fn convert_if_stmt(if_stmt: &IfStmt, context: &mut Context) -> String {
    let cond = convert_expr(&if_stmt.cond, context);
    let body = convert_block_stmt(&if_stmt.body, context);
    let else_ = if let Some(else_) = &if_stmt.else_ {
        let else_ = convert_stmt(else_, context);
        // The else statement is either an if statement or a block statement so the formatting will work the same in rust as it will for go with braces and we don't need to do anything special.
        format!(" else {else_}")
    } else {
        String::new()
    };

    if let Some(init) = &if_stmt.init {
        let init = convert_stmt(init, context);

        // We must put the init statement and following if block in it's own scope as variables declared in go init statements are local to the scope of the if statement they are declared for, hence the extra surrounding braces.
        format!("{{\n{init}\nif {cond} {body}{else_}\n}}")
    } else {
        format!("if {cond} {body}{else_}")
    }
}

fn convert_for_stmt(for_stmt: &ForStmt, context: &mut Context) -> String {
    let (loop_keyword, condition) = if let Some(cond) = &for_stmt.cond {
        ("while", convert_stmt(cond, context))
    } else {
        ("loop", String::new())
    };

    let mut body = convert_block_stmt(&for_stmt.body, context);
    if let Some(post) = &for_stmt.post {
        let post = convert_stmt(post, context);
        // We run the post condition at the end of the body, it's not as clean but it does the same job
        // TODO Check how this will work with break statements
        body = format!("{{\n{body}\n{post}\n}}");
    }

    let loop_stmt = format!("{loop_keyword} {condition}{body}");

    if let Some(init) = &for_stmt.init {
        let init = convert_stmt(init, context);
        // We must put the init statement and following for block in it's own scope as variables declared in go init statements are local to the scope of the for statement they are declared for, hence the extra surrounding braces.
        format!("{{\n{init}\n{loop_stmt}\n}}")
    } else {
        loop_stmt
    }
}

fn convert_range_stmt(range_stmt: &RangeStmt, context: &mut Context) -> String {
    let key = range_stmt
        .key
        .as_ref()
        .map(|key| convert_expr(key, context));

    let val = range_stmt
        .value
        .as_ref()
        .map(|val| convert_expr(val, context));

    let decl = match (key, val) {
        (Some(key), Some(val)) => format!("({key}, {val})"),
        (Some(var), None) | (None, Some(var)) => var,
        (None, None) => panic!("Cannot have range loop without a key or value"),
    };

    let mut expr = convert_expr(&range_stmt.expr, context);
    let mut body = convert_block_stmt(&range_stmt.body, context);

    // TODO We need to find a way to assign the variables outside of the loop scope if a simple "=" operator is used instead of ":=" for the range expression.
    format!("for {decl} in {expr} {body}")
}

fn convert_branch_stmt(branch: &BranchStmt, context: &mut Context) -> String {
    let label = branch
        .ident
        .as_ref()
        .map(|label| format!(" '{}", label.name))
        .unwrap_or_default();
    let control_flow = match &branch.key {
        Keyword::Continue => "continue",
        Keyword::Break => "break",
        other => panic!("Unexpected keyword {other:?} found in branch statement"),
    };

    format!("{control_flow}{label};")
}

fn convert_expr_switch_stmt(switch: &ExprSwitchStmt, context: &mut Context) -> String {
    let convert_to_if = switch.tag.is_none()
        || !switch.block.body.iter().all(|case| {
            case.list
                .iter()
                .all(|expr| matches!(expr, Expression::Ident(_) | Expression::BasicLit(_)))
        });
    let init = switch.init.as_ref().map(|init| convert_stmt(init, context));
    let switch = if convert_to_if {
        let tag = switch
            .tag
            .as_ref()
            .map(|expr| convert_expr(expr, context))
            .unwrap_or_else(|| "true".to_string());
        let mut cases = vec![];
        for case in &switch.block.body {
            let mut conditions = case
                .list
                .iter()
                .map(|cond| convert_expr(cond, context))
                .map(|cond| format!("({cond}) == {tag}"))
                .collect::<Vec<_>>()
                .join(" || ");
            let block = case
                .body
                .iter()
                .map(|stmt| convert_stmt(stmt, context))
                .collect::<Vec<_>>()
                .join(";\n");
            let if_keyword = match case.tok {
                Keyword::Case => "if ",
                Keyword::Default => "",
                other => panic!("Found unexpected keyword used for switch statement: {other:?}",),
            };
            cases.push(format!("{if_keyword}{conditions} {{\n{block}\n}}"))
        }
        cases.join(" else ")
    } else {
        // Unwrap is safe here as we have already checked that tag exists when checking if we should convert to an if statement or not.
        let tag = switch.tag.as_ref().unwrap();
        let tag = convert_expr(tag, context);
        let mut cases = vec![];
        for case in &switch.block.body {
            let mut conditions = case
                .list
                .iter()
                .map(|cond| convert_expr(cond, context))
                .collect::<Vec<_>>()
                .join(" | ");
            let block = case
                .body
                .iter()
                .map(|stmt| convert_stmt(stmt, context))
                .collect::<Vec<_>>()
                .join(";\n");
            // We don't know what statement finishes the block and some, like return statements require a trailing semicolon in match branches so we take the simple route and just wrap all branches in braces.
            let block = format!("{{\n{block}\n}}");
            let conditions = match case.tok {
                Keyword::Case => conditions,
                Keyword::Default => "_".to_string(),
                other => panic!("Found unexpected keyword used for switch statement: {other:?}",),
            };
            cases.push(format!("{conditions} => {block}"))
        }
        let cases = cases.join("\n");
        format!("match {tag} {{\n{cases}\n}}")
    };
    if let Some(init) = init {
        format!("{{\n{init};\n{switch}\n}}")
    } else {
        switch
    }
}
