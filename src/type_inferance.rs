use std::{cell::RefCell, rc::Rc};

use gosyn::token::{LitKind, Operator};

use crate::ast::{Operation, Type};
use crate::{
    ast::{
        BlockStmt, ConstSpec, Decl, Declaration, Expression, File, FuncDecl, Ident, Statement,
        VarSpec,
    },
    name_resolution::{Context, GoType, Scope},
};

pub fn infer_types_for_file(ast: &File, scope: Rc<RefCell<Scope>>) {
    let mut context = Context::new(scope);
    let mut context = context.next_scope().unwrap();
    for decl in &ast.decl {
        match decl {
            Declaration::Function(func_decl) => infer_types_for_func_decl(func_decl, &mut context),
            Declaration::Type(decl) => infer_types_for_type_decl(decl, &mut context),
            Declaration::Const(decl) => infer_types_for_const_decl(decl, &mut context),
            Declaration::Variable(decl) => infer_types_for_var_decl(decl, &mut context),
        }
    }
}

fn infer_types_for_const_decl(decl: &Decl<ConstSpec>, context: &mut Context) {
    for spec in &decl.specs {
        for (name, value) in spec.name.iter().zip(&spec.values) {
            if context.get_local_variable(&name.name).unwrap().is_unknown() {
                context.set_local_variable(
                    name.name.as_str(),
                    infer_type_for_expression(value, context),
                );
            };
        }
    }
}

fn infer_types_for_func_decl(func_decl: &FuncDecl, context: &mut Context) {
    // this type_decl should already be correctly added to the program scope so we don't need to do that.
    let mut context = context.next_scope().unwrap();
    if let Some(body) = &func_decl.body {
        infer_types_in_block_statement(body, &mut context);
    }
}

fn infer_types_in_block_statement(block: &BlockStmt, mut context: &mut Context) {
    for stmt in &block.list {
        infer_types_in_stmt(stmt, context);
    }
}

fn infer_types_in_stmt(stmt: &Statement, context: &mut Context) {
    match stmt {
        Statement::Go(go_stmt) => todo!(),
        Statement::If(if_stmt) => todo!(),
        Statement::For(for_stmt) => todo!(),
        Statement::Send(send_stmt) => todo!(),
        Statement::Expr(expr_stmt) => todo!(),
        Statement::Defer(defer_stmt) => todo!(),
        Statement::Block(block_stmt) => {
            let mut context = context.next_scope().unwrap();
            infer_types_in_block_statement(block_stmt, &mut context)
        }
        Statement::Range(range_stmt) => todo!(),
        Statement::Label(labeled_stmt) => todo!(),
        Statement::IncDec(inc_dec_stmt) => todo!(),
        Statement::Assign(assign_stmt) => todo!(),
        Statement::Return(return_stmt) => todo!(),
        Statement::Switch(switch_stmt) => todo!(),
        Statement::Select(select_stmt) => todo!(),
        Statement::Declaration(decl_stmt) => todo!(),
        Statement::ShortVarDecl(short_var_decl) => todo!(),
        Statement::Empty(_) | Statement::Branch(_) => {}
    }
}

fn infer_types_for_type_decl(decl: &Decl<crate::ast::TypeSpec>, context: &mut Context) {
    todo!()
}

fn infer_types_for_var_decl(decl: &Decl<VarSpec>, context: &mut Context) {
    for spec in &decl.specs {
        if spec.name.len() == spec.values.len() {
            for (name, value) in spec.name.iter().zip(&spec.values) {
                if context.get_local_variable(&name.name).unwrap().is_unknown() {
                    context.set_local_variable(
                        name.as_str(),
                        infer_type_for_expression(value, context),
                    );
                };
            }
        } else {
            todo![
                "Cannot yet infer types for functions with multiple return types or type assertions, got declaration {spec:?}"
            ]
        }
    }
}

pub fn infer_type_for_expression(expr: &Expression, context: &Context) -> GoType {
    match expr {
        Expression::Star(expr) => infer_type_for_expression(&*expr.right, context),
        Expression::Call(call) => {
            let func_type = infer_type_for_expression(&call.func, context);
            match func_type {
                GoType::Declared(Type::Function(func_type)) | GoType::Function(func_type) => {
                    let result = func_type
                        .result
                        .iter()
                        .flat_map(|param| {
                            let typ = param.typ.clone();
                            if param.identifiers.len() == 0 {
                                vec![typ]
                            } else {
                                param.identifiers.iter().map(|_| typ.clone()).collect()
                            }
                        })
                        .collect::<Vec<_>>();
                    if result.len() == 0 {
                        unreachable![
                            "Attempted to infer type of call to function with no return type"
                        ]
                    } else if result.len() == 1 {
                        GoType::Declared(result[0].clone())
                    } else {
                        GoType::List(result)
                    }
                }
                GoType::Unknown => {
                    return GoType::Unknown;
                }
                other => {
                    unreachable!("Attempted to infer call type of a non-function, got: {other:?}")
                }
            }
        }
        Expression::Index(index) => todo!(),
        Expression::IndexList(index_list) => todo!(),
        Expression::Slice(slice) => todo!(),
        Expression::Ident(ident) => resolve_type(ident, context),
        Expression::FuncLit(func_lit) => GoType::Function(func_lit.typ.clone()),
        Expression::Ellipsis(ellipsis) => todo!(),
        Expression::Selector(selector) => todo!(),
        Expression::BasicLit(basic_lit) => match basic_lit.kind {
            // Inline resolve_type here because we don't actually have an `Ident`, I suspect this branch is never ran but best to be safe.
            LitKind::Ident => context
                .get_variable(basic_lit.value.as_str())
                .unwrap_or_default(),

            LitKind::String => GoType::String,
            LitKind::Integer => GoType::Int,
            LitKind::Float => GoType::Float,
            LitKind::Imag => GoType::Imag,
            LitKind::Char => GoType::Char,
        },
        Expression::Range(range_expr) => todo!(),
        Expression::Paren(expr) => infer_type_for_expression(&*expr.expr, context),
        Expression::TypeAssert(type_assertion) => todo!(),
        Expression::CompositeLit(composite_lit) => todo!(),
        Expression::List(expressions) => todo!(),
        Expression::Operation(op) => infer_type_for_operation(op, context),
        Expression::Type(typ) => GoType::Type(typ.clone()),
    }
}

pub fn infer_type_for_operation(op: &Operation, context: &Context) -> GoType {
    match op.op {
        Operator::NotEqual
        | Operator::Less
        | Operator::LessEqual
        | Operator::Greater
        | Operator::GreaterEqual
        | Operator::AndAnd
        | Operator::OrOr
        | Operator::Equal
        | Operator::Not => GoType::Bool,
        Operator::Add => {
            let left_type = infer_type_for_expression(&op.x, context);
            let right_type = infer_type_for_expression(op.y.as_ref().unwrap(), context);
            match (left_type, right_type) {
                (GoType::Unknown, typ) | (typ, GoType::Unknown) => typ,
                (GoType::Int, GoType::Int) => GoType::Int,
                (GoType::String, GoType::String) => GoType::String,
                (GoType::Float, GoType::Float) => GoType::Float,
                (GoType::Imag, GoType::Imag) => GoType::Imag,
                _ => GoType::Unknown,
            }
        }
        Operator::Sub => {
            let x_type = infer_type_for_expression(&op.x, context);
            let Some(y) = &op.y else {
                return x_type;
            };
            let y_type = infer_type_for_expression(y, context);
            match (x_type, y_type) {
                (GoType::Unknown, typ) | (typ, GoType::Unknown) => typ,
                (GoType::Int, GoType::Int) => GoType::Int,
                (GoType::Float, GoType::Float) => GoType::Float,
                (GoType::Imag, GoType::Imag) => GoType::Imag,
                _ => GoType::Unknown,
            }
        }
        Operator::AndNot | Operator::Or | Operator::Shl | Operator::Shr | Operator::Xor => {
            GoType::Int
        }
        Operator::And => {
            if op.y.is_some() {
                // Bit and
                GoType::Int
            } else {
                // Address
                GoType::Pointer(Box::new(infer_type_for_expression(&op.x, context)))
            }
        }
        Operator::Star | Operator::Quo => {
            let left_type = infer_type_for_expression(&op.x, context);
            let right_type = infer_type_for_expression(op.y.as_ref().unwrap(), context);
            match (left_type, right_type) {
                (GoType::Unknown, typ) | (typ, GoType::Unknown) => typ,
                (typ, _) => typ,
            }
        }
        Operator::Rem => GoType::Int,
        Operator::Arrow => todo!(),
        other => {
            unreachable!["Found unexpected operator type during type inference, got {other:?}"]
        }
    }
}

fn resolve_type(ident: &Ident, context: &Context) -> GoType {
    context
        .get_variable(ident.name.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod test {
    use std::{assert_matches, cell::RefCell, rc::Rc};

    use gosyn::Parser;

    use crate::{
        ast::{NameType, Type},
        name_resolution::{Context, GoType, find_use_idents},
    };

    use super::infer_types_for_file;

    fn infer_types_for(src: &'static str) -> Context {
        let mut parser = Parser::from(src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap()
            .into();
        let mut scope = Rc::new(RefCell::new(find_use_idents(&file)));
        infer_types_for_file(&file, scope.clone());
        Context::new(scope)
    }

    #[test]
    fn var_with_value_true_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool=true");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_false_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool=false");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_iota_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar is_bool=iota");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Int));
    }

    #[test]
    fn var_with_int_value_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar is_int=5");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_float_value_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar pi=3.1415");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("pi"), Some(GoType::Float));
    }

    #[test]
    fn var_with_imaginary_value_is_infered_as_imaginary() {
        let mut context = infer_types_for("package main\nvar complex = 5i");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("complex"), Some(GoType::Imag));
    }

    #[test]
    fn var_with_string_value_is_infered_as_string() {
        let mut context = infer_types_for("package main\nvar is_string=\"hello world\"");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_string"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_less_than_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a < b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_greater_than_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a > b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_less_than_or_equal_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a <= b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_greater_than_or_equal_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a >= b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_equal_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a == b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_not_equal_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a != b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_and_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a == b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_or_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= a || b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_not_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar is_bool= !a");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    // Only one test here to check that constants are being infered as expected.
    // Most of the above tests are more testing the correctness of the inference for different operations which is unrelated to the actual declaration being infered.
    #[test]
    fn const_with_value_true_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nconst is_bool=true");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_string_addition_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar str= \"hello\" + \"world\"");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_string_and_unknown_addition_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar str= \"hello\" + world");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_unknown_and_string_addition_operation_is_infered_as_bool() {
        let mut context = infer_types_for("package main\nvar str= hello + \"world\"");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_integer_addition_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 1 + 2");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_int_and_unknown_addition_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 2 + unknown");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_unknown_and_integer_addition_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= unknown + 3");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_float_addition_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 1.0 + 2.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_float_and_unknown_addition_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 2.0 + unknown");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_unknown_and_float_addition_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= unknown + 3.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_integer_subtraction_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 1 - 2");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_int_and_unknown_subtraction_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 2 - unknown");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_unknown_and_integer_subtraction_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= unknown - 3");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_float_subtraction_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 1.0 - 2.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_float_and_unknown_subtraction_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 2.0 - unknown");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_unknown_and_float_subtraction_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= unknown - 3.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_negative_float_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= -3.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_negative_int_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= -3");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_notand_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a&^b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_and_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a&b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_xor_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a^b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_or_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a|b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_rhs_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a>>b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_lhs_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a<<b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_rem_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a%b");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_int_mul_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 3*2");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_int_and_unknown_mul_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 3*a");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_unknown_and_int_mul_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a*3");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_float_mul_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 3.0*2.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_float_and_unknown_mul_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 3.0*a");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_unknown_and_float_mul_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= a*3.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_int_quo_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 3/2");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_int_and_unknown_quo_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= 3/a");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_unknown_and_int_quo_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar int= a/3");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_float_quo_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 3.0/2.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_float_and_unknown_quo_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= 3.0/a");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_unknown_and_float_quo_operation_is_infered_as_float() {
        let mut context = infer_types_for("package main\nvar float= a/3.0");
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("float"), Some(GoType::Float));
    }

    #[test]
    fn var_with_value_address_to_int_operation_is_infered_as_pointer_to_int() {
        let mut context = infer_types_for("package main\nvar int= &3");
        let file_context = context.next_scope().unwrap();
        assert_matches!(
            file_context.get_variable("int"),
            Some(GoType::Pointer(typ)) if typ.is_int()
        );
    }

    #[test]
    fn var_with_value_dereference_for_pointer_int_operation_is_infered_as_int() {
        let mut context = infer_types_for("package main\nvar intptr= &3\nvar int = *intptr");
        let file_context = context.next_scope().unwrap();
        assert_matches!(
            file_context.get_variable("int"),
            Some(GoType::Pointer(typ)) if typ.is_int()
        );
    }

    #[test]
    fn var_with_value_from_call_of_int_is_infered_as_int() {
        let mut context = infer_types_for("package main\nfunc getint()int{}\nvar int = getint()");
        let file_context = context.next_scope().unwrap();
        assert_matches!(
            file_context.get_variable("int"),
            Some(GoType::Declared(Type::Name(NameType::Ident(name)))) if name.as_str()=="int"
        );
    }
}
