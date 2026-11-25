use gosyn::token::{LitKind, Operator};

use crate::{
    ast::{
        ConstSpec, Decl, Declaration, Expression, File, FuncDecl, Ident, NameType, Operation, Type,
        VarSpec,
    },
    name_resolution::{Context, GoType, Scope},
};

pub fn infer_types_for_file(ast: &File, scope: &mut Scope) {
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

fn infer_types_for_const_decl(decl: &Decl<ConstSpec>, context: &mut Context<'_>) {
    for spec in &decl.specs {
        for (name, value) in spec.name.iter().zip(&spec.values) {
            if context.scope.get_variable(&name.name).unwrap().is_unknown() {
                *context.scope.variables.get_mut(name.name.as_str()).unwrap() =
                    infer_type_for_expression(value, context);
            };
        }
    }
}

fn infer_types_for_func_decl(func_decl: &FuncDecl, context: &mut Context<'_>) {
    todo!()
}

fn infer_types_for_type_decl(decl: &Decl<crate::ast::TypeSpec>, context: &mut Context<'_>) {
    todo!()
}

fn infer_types_for_var_decl(decl: &Decl<VarSpec>, context: &mut Context<'_>) {
    for spec in &decl.specs {
        for (name, value) in spec.name.iter().zip(&spec.values) {
            if context.scope.get_variable(&name.name).unwrap().is_unknown() {
                *context.scope.variables.get_mut(name.name.as_str()).unwrap() =
                    infer_type_for_expression(value, context);
            };
        }
    }
}

pub fn infer_type_for_expression(expr: &Expression, context: &Context) -> GoType {
    match expr {
        Expression::Star(expr) => infer_type_for_expression(&*expr.right, context),
        Expression::Call(call) => todo!(),
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
                .cloned()
                .unwrap_or(GoType::Unknown),

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
                (GoType::Int, GoType::Int)
                | (GoType::Int, GoType::Unknown)
                | (GoType::Unknown, GoType::Int) => GoType::Int,
                (GoType::String, GoType::String)
                | (GoType::String, GoType::Unknown)
                | (GoType::Unknown, GoType::String) => GoType::String,
                _ => GoType::Unknown,
            }
        }
        _ => unimplemented![],
    }
}

fn resolve_type(ident: &Ident, context: &Context) -> GoType {
    context
        .get_variable(ident.name.as_str())
        .cloned()
        .unwrap_or(GoType::Unknown)
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use gosyn::Parser;

    use crate::{
        ast::{NameType, Type},
        name_resolution::{Context, GoType, find_use_idents},
    };

    use super::{Scope, infer_types_for_file};

    fn infer_types_for(src: &'static str) -> Scope {
        let mut parser = Parser::from(src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap()
            .into();

        let mut scope = find_use_idents(&file);
        infer_types_for_file(&file, &mut scope);
        scope
    }

    #[test]
    fn var_with_value_true_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool=true");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_false_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool=false");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_iota_is_infered_as_int() {
        let mut scope = infer_types_for("package main\nvar is_bool=iota");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Int));
    }

    #[test]
    fn var_with_int_value_is_infered_as_int() {
        let mut scope = infer_types_for("package main\nvar is_int=5");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_int"), Some(GoType::Int));
    }

    #[test]
    fn var_with_float_value_is_infered_as_float() {
        let mut scope = infer_types_for("package main\nvar pi=3.1415");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("pi"), Some(GoType::Float));
    }

    #[test]
    fn var_with_string_value_is_infered_as_string() {
        let mut scope = infer_types_for("package main\nvar is_string=\"hello world\"");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_string"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_less_than_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a < b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_greater_than_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a > b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_less_than_or_equal_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a <= b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_greater_than_or_equal_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a >= b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_equal_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a == b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_not_equal_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a != b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_and_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a == b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_or_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= a || b");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_logical_not_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar is_bool= !a");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    // Only one test here to check that constants are being infered as expected.
    // Most of the above tests are more testing the correctness of the inference for different operations which is unrelated to the actual declaration being infered.
    #[test]
    fn const_with_value_true_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nconst is_bool=true");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("is_bool"), Some(GoType::Bool));
    }

    #[test]
    fn var_with_value_string_addition_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar str= \"hello\" + \"world\"");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_string_and_unknown_addition_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar str= \"hello\" + world");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_unknown_and_string_addition_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar str= hello + \"world\"");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::String));
    }

    #[test]
    fn var_with_value_integer_addition_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar str= 1 + 2");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_int_and_unknown_addition_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar str= 2 + unknown");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::Int));
    }

    #[test]
    fn var_with_value_unknown_and_integer_addition_operation_is_infered_as_bool() {
        let mut scope = infer_types_for("package main\nvar str= unknown + 3");
        let mut context = Context::new(&mut scope);
        let file_context = context.next_scope().unwrap();
        assert_matches!(file_context.get_variable("str"), Some(GoType::Int));
    }
}
