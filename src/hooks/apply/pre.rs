use crate::{
    ast::{
        BlockStmt, Call, ConstSpec, Decl, DeclStmt, Declaration, Element, Expression, Field, File,
        FuncType, InterfaceElem, KeyedElement, LiteralValue, Statement, SwitchStmt, Type,
        TypeParamDecl, TypeSpec, TypeTerm, VarSpec,
    },
    name_resolution::Context,
};

pub struct Hooks {
    pub types: Vec<&'static dyn Fn(&mut Type, &mut Context)>,
    pub func_types: Vec<&'static dyn Fn(&mut FuncType, &mut Context)>,
    pub statements: Vec<&'static dyn Fn(&mut Statement, &mut Context)>,
    pub expressions: Vec<&'static dyn Fn(&mut Expression, &mut Context)>,
}

pub fn apply(ast: &mut File, context: &mut Context, hooks: &Hooks) {
    for decl in &mut ast.decl {
        match decl {
            Declaration::Type(type_decl) => apply_for_type_decl(type_decl, context, hooks),
            Declaration::Variable(var_decl) => apply_for_variable_decl(var_decl, context, hooks),
            Declaration::Const(const_decl) => apply_for_const_decl(const_decl, context, hooks),
            Declaration::Function(func_decl) => {
                apply_for_func_type(&mut func_decl.typ, context, hooks);
                if let Some(body) = &mut func_decl.body {
                    apply_for_block_statement(body, context, hooks);
                }
            }
        }
    }
}

fn apply_for_type(typ: &mut Type, context: &mut Context, hooks: &Hooks) {
    match typ {
        Type::Instantiated(instantiated_type) => {
            for argument in &mut instantiated_type.arguments {
                apply_for_type(argument, context, hooks)
            }
        }
        Type::Map(map_type) => {
            apply_for_type(map_type.key.as_mut(), context, hooks);
            apply_for_type(map_type.val.as_mut(), context, hooks);
        }
        Type::Array(array_type) => apply_for_type(array_type.typ.as_mut(), context, hooks),
        Type::Slice(slice_type) => apply_for_type(slice_type.typ.as_mut(), context, hooks),
        Type::Function(func_type) => apply_for_func_type(func_type, context, hooks),
        Type::Struct(struct_type) => {
            for field in &mut struct_type.fields {
                match &mut field.field {
                    Field::Named(named_field) => {
                        apply_for_type(&mut named_field.typ, context, hooks)
                    }
                    Field::Embedded(embedded_field) => {
                        // These fields can only be named types or pointers to named types so we cannot apply the hook to the field type directly.
                        // We can apply it to the type arguments if any however.
                        for type_param in &mut embedded_field.type_args {
                            apply_for_type(type_param, context, hooks)
                        }
                    }
                }
            }
        }
        Type::Channel(channel_type) => apply_for_type(channel_type.typ.as_mut(), context, hooks),
        Type::Pointer(pointer_type) => apply_for_type(pointer_type.typ.as_mut(), context, hooks),
        Type::Interface(interface_type) => {
            for element in &mut interface_type.elements {
                match element {
                    InterfaceElem::Method(method_elem) => {
                        apply_for_func_type(&mut method_elem.typ, context, hooks)
                    }
                    InterfaceElem::Type(type_elem) => {
                        for term in &mut type_elem.types {
                            apply_for_type_term(term, context, hooks)
                        }
                    }
                }
            }
        }
        Type::Paren(paren_type) => apply_for_type(paren_type.typ.as_mut(), context, hooks),
        Type::Name(_) => {
            // There are no types inside named types so we do nothing.
        }
    }
    for hook in &hooks.types {
        hook(typ, context);
    }
}

fn apply_for_type_term(term: &mut TypeTerm, context: &mut Context, hooks: &Hooks) {
    match term {
        TypeTerm::Type(typ) => apply_for_type(typ, context, hooks),
        TypeTerm::UnderLyingType(under_lying_type) => {
            apply_for_type(under_lying_type.typ.as_mut(), context, hooks)
        }
    }
}

fn apply_for_statement(stmt: &mut Statement, context: &mut Context, hooks: &Hooks) {
    match stmt {
        Statement::Go(go_stmt) => apply_for_call(&mut go_stmt.call, context, hooks),
        Statement::If(if_stmt) => {
            if let Some(init) = if_stmt.init.as_deref_mut() {
                apply_for_statement(init, context, hooks)
            }
            apply_for_expression(&mut if_stmt.cond, context, hooks);
            apply_for_block_statement(&mut if_stmt.body, context, hooks);
            if let Some(else_) = if_stmt.else_.as_deref_mut() {
                apply_for_statement(else_, context, hooks);
            }
        }
        Statement::For(for_stmt) => {
            if let Some(init) = for_stmt.init.as_deref_mut() {
                apply_for_statement(init, context, hooks);
            }
            if let Some(cond) = for_stmt.cond.as_deref_mut() {
                apply_for_statement(cond, context, hooks);
            }
            if let Some(post) = for_stmt.post.as_deref_mut() {
                apply_for_statement(post, context, hooks);
            }
            apply_for_block_statement(&mut for_stmt.body, context, hooks);
        }
        Statement::Send(send_stmt) => {
            apply_for_expression(&mut send_stmt.chan, context, hooks);
            apply_for_expression(&mut send_stmt.value, context, hooks);
        }
        Statement::Expr(expr_stmt) => apply_for_expression(&mut expr_stmt.expr, context, hooks),
        Statement::Defer(defer_stmt) => apply_for_call(&mut defer_stmt.call, context, hooks),
        Statement::Block(block_stmt) => apply_for_block_statement(block_stmt, context, hooks),
        Statement::Range(range_stmt) => {
            if let Some(key) = &mut range_stmt.key {
                apply_for_expression(key, context, hooks);
            }
            if let Some(value) = &mut range_stmt.value {
                apply_for_expression(value, context, hooks);
            }
            apply_for_expression(&mut range_stmt.expr, context, hooks);
            apply_for_block_statement(&mut range_stmt.body, context, hooks);
        }
        Statement::Label(labeled_stmt) => {
            apply_for_statement(labeled_stmt.stmt.as_mut(), context, hooks)
        }
        Statement::IncDec(inc_dec_stmt) => {
            apply_for_expression(&mut inc_dec_stmt.expr, context, hooks)
        }
        Statement::Assign(assign_stmt) => {
            for left in &mut assign_stmt.left {
                apply_for_expression(left, context, hooks);
            }
            for right in &mut assign_stmt.right {
                apply_for_expression(right, context, hooks);
            }
        }
        Statement::Return(return_stmt) => {
            for ret in &mut return_stmt.ret {
                apply_for_expression(ret, context, hooks);
            }
        }
        Statement::Switch(SwitchStmt::Type(type_switch)) => {
            if let Some(init) = type_switch.init.as_deref_mut() {
                apply_for_statement(init, context, hooks);
            }
            if let Some(tag) = &mut type_switch.tag {
                apply_for_statement(tag, context, hooks);
            }
            for case in &mut type_switch.block.body {
                for typ in &mut case.list {
                    apply_for_type(typ, context, hooks);
                }
                for stmt in &mut case.body {
                    apply_for_statement(stmt, context, hooks);
                }
            }
        }
        Statement::Switch(SwitchStmt::Expr(expr_switch)) => {
            if let Some(init) = expr_switch.init.as_deref_mut() {
                apply_for_statement(init, context, hooks);
            }
            if let Some(tag) = &mut expr_switch.tag {
                apply_for_expression(tag, context, hooks);
            }
            for case in &mut expr_switch.block.body {
                for expr in &mut case.list {
                    apply_for_expression(expr, context, hooks);
                }
                for stmt in &mut case.body {
                    apply_for_statement(stmt, context, hooks);
                }
            }
        }
        Statement::Select(select_stmt) => {
            for clause in &mut select_stmt.body.body {
                if let Some(comm) = clause.comm.as_deref_mut() {
                    apply_for_statement(comm, context, hooks)
                }
                for stmt in &mut clause.body {
                    apply_for_statement(stmt, context, hooks)
                }
            }
        }
        Statement::Declaration(decl_stmt) => match decl_stmt {
            DeclStmt::Type(decl) => apply_for_type_decl(decl, context, hooks),
            DeclStmt::Const(decl) => apply_for_const_decl(decl, context, hooks),
            DeclStmt::Variable(decl) => apply_for_variable_decl(decl, context, hooks),
        },
        Statement::ShortVarDecl(short_var_decl) => {
            for value in &mut short_var_decl.values {
                apply_for_expression(value, context, hooks);
            }
        }
        Statement::Branch(_) | Statement::Empty(_) => {
            // No types inside these statement types so we do nothing.
        }
    }
    for hook in &hooks.statements {
        hook(stmt, context);
    }
}

fn apply_for_variable_decl(var_decl: &mut Decl<VarSpec>, context: &mut Context, hooks: &Hooks) {
    for spec in &mut var_decl.specs {
        if let Some(typ) = &mut spec.typ {
            apply_for_type(typ, context, hooks)
        }
    }
}

fn apply_for_const_decl(const_decl: &mut Decl<ConstSpec>, context: &mut Context, hooks: &Hooks) {
    for spec in &mut const_decl.specs {
        if let Some(typ) = &mut spec.typ {
            apply_for_type(typ, context, hooks)
        }
    }
}

fn apply_for_type_decl(type_decl: &mut Decl<TypeSpec>, context: &mut Context, hooks: &Hooks) {
    for spec in &mut type_decl.specs {
        for type_param in &mut spec.params {
            apply_for_type_param_decl(type_param, context, hooks);
        }
        apply_for_type(&mut spec.typ, context, hooks);
    }
}

fn apply_for_block_statement(block: &mut BlockStmt, context: &mut Context, hooks: &Hooks) {
    for stmt in &mut block.list {
        apply_for_statement(stmt, context, hooks);
    }
}

fn apply_for_expression(expr: &mut Expression, context: &mut Context, hooks: &Hooks) {
    match expr {
        Expression::Call(call) => apply_for_call(call, context, hooks),
        Expression::Index(index) => {
            apply_for_expression(index.left.as_mut(), context, hooks);
            apply_for_expression(index.index.as_mut(), context, hooks);
        }
        Expression::IndexList(index_list) => {
            apply_for_expression(index_list.left.as_mut(), context, hooks);
            for index in &mut index_list.indices {
                apply_for_expression(index, context, hooks);
            }
        }
        Expression::Slice(slice) => {
            apply_for_expression(slice.left.as_mut(), context, hooks);
            for index in &mut slice.index {
                if let Some(index) = index.as_deref_mut() {
                    apply_for_expression(index, context, hooks);
                }
            }
        }
        Expression::FuncLit(func_lit) => {
            apply_for_func_type(&mut func_lit.typ, context, hooks);
            apply_for_block_statement(&mut func_lit.body, context, hooks);
        }
        Expression::Ellipsis(ellipsis) => {
            if let Some(elt) = ellipsis.elt.as_deref_mut() {
                apply_for_expression(elt, context, hooks)
            }
        }
        Expression::Selector(selector) => apply_for_expression(selector.x.as_mut(), context, hooks),
        Expression::Range(range_expr) => {
            apply_for_expression(range_expr.right.as_mut(), context, hooks)
        }
        Expression::Star(star_expr) => {
            apply_for_expression(star_expr.right.as_mut(), context, hooks)
        }
        Expression::Paren(paren) => apply_for_expression(paren.expr.as_mut(), context, hooks),
        Expression::TypeAssert(type_assertion) => {
            apply_for_expression(type_assertion.left.as_mut(), context, hooks);
            if let Some(right) = type_assertion.right.as_deref_mut() {
                apply_for_type(right, context, hooks);
            }
        }
        Expression::CompositeLit(composite_lit) => {
            apply_for_expression(composite_lit.typ.as_mut(), context, hooks);
            apply_for_literal_value(&mut composite_lit.val, context, hooks)
        }
        Expression::List(expressions) => {
            for expr in expressions {
                apply_for_expression(expr, context, hooks);
            }
        }
        Expression::Operation(operation) => {
            apply_for_expression(operation.x.as_mut(), context, hooks);
            if let Some(y) = operation.y.as_deref_mut() {
                apply_for_expression(y, context, hooks);
            }
        }
        Expression::Type(typ) => apply_for_type(typ, context, hooks),
        Expression::BasicLit(_) | Expression::Ident(_) => {
            // These expressions do not have types inside them so we do nothing.
        }
    }
    for hook in &hooks.expressions {
        hook(expr, context);
    }
}

fn apply_for_literal_value(val: &mut LiteralValue, context: &mut Context, hooks: &Hooks) {
    for KeyedElement { key, val } in &mut val.values {
        if let Some(key) = key {
            apply_for_element(key, context, hooks)
        }
        apply_for_element(val, context, hooks)
    }
}

fn apply_for_element(element: &mut Element, context: &mut Context, hooks: &Hooks) {
    match element {
        Element::Expr(expression) => apply_for_expression(expression, context, hooks),
        Element::LitValue(literal_value) => apply_for_literal_value(literal_value, context, hooks),
    }
}

fn apply_for_func_type(func_type: &mut FuncType, context: &mut Context, hooks: &Hooks) {
    for type_param in &mut func_type.typ_params {
        apply_for_type_param_decl(type_param, context, hooks);
    }
    for param in &mut func_type.params {
        apply_for_type(&mut param.typ, context, hooks);
    }
    for result in &mut func_type.result {
        apply_for_type(&mut result.typ, context, hooks);
    }
    for hook in &hooks.func_types {
        hook(func_type, context);
    }
}

fn apply_for_type_param_decl(type_param: &mut TypeParamDecl, context: &mut Context, hooks: &Hooks) {
    for constraint in &mut type_param.constraint.types {
        apply_for_type_term(constraint, context, hooks);
    }
}

fn apply_for_call(call: &mut Call, context: &mut Context, hooks: &Hooks) {
    for argument in &mut call.args {
        apply_for_expression(argument, context, hooks);
    }
    apply_for_expression(call.func.as_mut(), context, hooks);
}
