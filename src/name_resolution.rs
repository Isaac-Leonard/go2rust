use std::collections::HashMap;

use crate::{
    ast::{
        BlockStmt, Call, Declaration, Expression, Field, File, FuncType, Statement, Type, TypeElem,
    },
    utils::VecExt,
};

fn find_use_idents(ast: &File) -> Scope {
    // TODO Have this be an over arching scope for all files and have proper package scopes.
    let mut top_scope = Scope::new_universe();
    let mut scope = Scope::default();
    for decl in &ast.decl {
        match decl {
            Declaration::Const(constants) => {
                for constant in &constants.specs {
                    let typ = if let Some(typ) = &constant.typ {
                        typ.clone().into()
                    } else {
                        GoType::Unknown
                    };

                    for name in &constant.name {
                        scope.provided.insert(name.name.clone(), typ.clone());
                    }
                }
            }
            Declaration::Variable(variables) => {
                for variables in &variables.specs {
                    let typ = if let Some(typ) = &variables.typ {
                        scope.add_sub_scopes(find_scopes_in_type(typ));
                        typ.clone().into()
                    } else {
                        GoType::Unknown
                    };

                    for name in &variables.name {
                        scope.add_variable(name.name.clone(), GoType::Unknown);
                    }

                    for value in &variables.values {
                        scope.add_sub_scopes(find_idents_in_expression(value));
                    }
                }
            }
            Declaration::Function(func_decl) => {
                scope.add_variable(
                    func_decl.name.name.clone(),
                    GoType::Function(func_decl.typ.clone()),
                );
                let mut sub_scope = get_scope_for_function_type(&func_decl.typ);
                // For if this function is a method
                // TODO Check if recv can actually have more then one value

                if let Some(body) = &func_decl.body {
                    find_idents_in_block(body, &mut sub_scope)
                }
                scope.add_sub_scope(sub_scope);
            }
            Declaration::Type(decl) => {
                for spec in &decl.specs {
                    scope.add_sub_scopes(find_scopes_in_type(&spec.typ));
                    scope.add_type(spec.name.name.clone(), spec.typ.clone());
                }
            }
        }
    }
    top_scope.add_sub_scope(scope);
    top_scope
}

fn find_idents_in_statement(statement: &Statement, scope: &mut Scope) {
    match statement {
        Statement::Go(go) => scope.add_sub_scopes(find_idents_in_call(&go.call)),
        Statement::If(if_stmt) => {
            let mut init_scope = Scope::default();
            if let Some(init) = &if_stmt.init {
                find_idents_in_statement(init, &mut init_scope);
            }
            init_scope.add_sub_scopes(find_idents_in_expression(&if_stmt.cond));
            let mut if_scope = Scope::default();
            find_idents_in_block(&if_stmt.body, &mut if_scope);
            init_scope.add_sub_scope(if_scope);
            if let Some(else_stmt) = &if_stmt.else_ {
                let mut else_scope = Scope::default();
                find_idents_in_statement(else_stmt, &mut else_scope);
                init_scope.add_sub_scope(else_scope)
            }
            scope.add_sub_scope(init_scope)
        }
        Statement::For(for_stmt) => todo!(),
        Statement::Send(send_stmt) => todo!(),
        Statement::Expr(expr_stmt) => todo!(),
        Statement::Defer(defer_stmt) => todo!(),
        Statement::Block(block_stmt) => todo!(),
        Statement::Range(range_stmt) => todo!(),
        Statement::Empty(empty_stmt) => todo!(),
        Statement::Label(labeled_stmt) => todo!(),
        Statement::IncDec(inc_dec_stmt) => todo!(),
        Statement::Assign(assign_stmt) => {
            assign_stmt.op;
        }
        Statement::Return(return_stmt) => todo!(),
        Statement::Branch(branch_stmt) => todo!(),
        Statement::Switch(switch_stmt) => todo!(),
        Statement::Select(select_stmt) => todo!(),
        Statement::Declaration(decl_stmt) => todo!(),
        Statement::ShortVarDecl(short_var_decl) => todo!(),
    }
}

fn find_idents_in_block(block: &BlockStmt, scope: &mut Scope) {
    for statement in &block.list {
        find_idents_in_statement(statement, scope);
    }
}

fn find_idents_in_expression(exp: &Expression) -> Vec<Scope> {
    match exp {
        Expression::Ident(ident) => vec![],
        Expression::BasicLit(lit) => vec![],
        Expression::Index(index) => {
            let left = find_idents_in_expression(&index.left);
            let right = find_idents_in_expression(&index.index);
            left.concat(right)
        }
        Expression::Operation(op) => {
            let left = find_idents_in_expression(&op.x);
            if let Some(right) = &op.y {
                let right = find_idents_in_expression(&right);
                left.concat(right)
            } else {
                left
            }
        }
        Expression::List(list) => {
            let mut idents = vec![];
            for exp in list {
                idents.extend(find_idents_in_expression(exp));
            }
            idents
        }
        Expression::Call(call) => find_idents_in_call(call),
        Expression::IndexList(index_list) => {
            let mut idents = find_idents_in_expression(&index_list.left);
            for index in &index_list.indices {
                idents.extend(find_idents_in_expression(&index));
            }
            idents
        }
        Expression::Slice(slice) => {
            let mut idents = find_idents_in_expression(&slice.left);
            for index in &slice.index {
                if let Some(index) = index {
                    idents.extend(find_idents_in_expression(&index));
                }
            }
            idents
        }
        Expression::FuncLit(func_lit) => todo!(),
        Expression::Ellipsis(ellipsis) => todo!(),
        Expression::Selector(selector) => {
            let mut idents = find_idents_in_expression(&selector.x);
            // idents.push(Referenced(selector.sel.name.clone()));
            idents
        }
        Expression::Range(range_expr) => find_idents_in_expression(&range_expr.right),
        Expression::Star(star_expression) => find_idents_in_expression(&star_expression.right),
        Expression::Paren(paren_expression) => find_idents_in_expression(&paren_expression.expr),
        Expression::TypeAssert(type_assertion) => {
            let left = find_idents_in_expression(&type_assertion.left);
            if let Some(right) = &type_assertion.right {
                let right = find_scopes_in_type(&right);
                left.concat(right)
            } else {
                left
            }
        }
        Expression::CompositeLit(composite_lit) => todo!(),
        Expression::Type(typ) => find_scopes_in_type(typ),
    }
}

fn find_scopes_in_type(typ: &Type) -> Vec<Scope> {
    match typ {
        Type::Map(map_type) => {
            let key = find_scopes_in_type(&map_type.key);
            let val = find_scopes_in_type(&map_type.val);
            key.concat(val)
        }
        Type::Array(array_type) => find_scopes_in_type(&array_type.typ),
        Type::Slice(slice_type) => find_scopes_in_type(&slice_type.typ),
        Type::Channel(channel_type) => find_scopes_in_type(&channel_type.typ),
        Type::Pointer(pointer_type) => find_scopes_in_type(&pointer_type.typ),
        Type::Paren(typ) => find_scopes_in_type(&typ.typ),
        Type::Name(_name_type) => vec![],
        Type::Instantiated(instantiated_type) => instantiated_type
            .arguments
            .iter()
            .flat_map(find_scopes_in_type)
            .collect(),
        Type::Struct(struct_type) => struct_type
            .fields
            .iter()
            .flat_map(|field| match &field.field {
                Field::Named(named_field) => find_scopes_in_type(&named_field.typ),
                Field::Embedded(_embedded_field) => vec![],
            })
            .collect(),
        Type::Interface(interface_type) => todo!(),
        Type::Function(func_type) => vec![get_scope_for_function_type(func_type)],
    }
}

fn get_scope_for_function_type(func_type: &FuncType) -> Scope {
    let mut scope = Scope::default();
    for type_param in &func_type.typ_params {
        let typ = GoType::TypeParam(type_param.constraint.clone());
        for name in &type_param.name {
            scope.add_type(name.name.clone(), typ.clone());
        }
    }

    for param in &func_type.params {
        scope.add_sub_scopes(find_scopes_in_type(&param.typ));
        let typ = GoType::Declared(param.typ.clone());
        for name in &param.identifiers {
            scope.add_variable(name.name.clone(), typ.clone());
        }
    }

    for param in &func_type.params {
        let typ = GoType::Declared(param.typ.clone());
        for name in &param.identifiers {
            scope.add_variable(name.name.clone(), typ.clone());
        }
    }

    for param in &func_type.result {
        let typ = GoType::Declared(param.typ.clone());
        for name in &param.identifiers {
            scope.add_variable(name.name.clone(), typ.clone());
        }
    }

    scope
}

pub fn find_idents_in_call(call: &Call) -> Vec<Scope> {
    let mut idents = find_idents_in_expression(&call.func);
    for arg in &call.args {
        idents.extend(find_idents_in_expression(arg));
    }
    idents
}

/// Contains lists of variables declared and used in a given program scope such as the entire program or a function.
#[derive(Debug, Default)]
pub struct Scope {
    //Named generics, or declared types in this scope
    pub types: HashMap<String, GoType>,
    /// function arguments or variables declared in this scope
    provided: HashMap<String, GoType>,
    /// Functions declared in this scope with their own arguments and local variables
    sub_scopes: Vec<Scope>,
}

impl Scope {
    pub fn new(
        types: HashMap<String, GoType>,
        provided: HashMap<String, GoType>,
        sub_scopes: Vec<Scope>,
    ) -> Self {
        Self {
            types,
            provided,
            sub_scopes,
        }
    }

    pub fn new_universe() -> Self {
        Self {
            types: [
                "any",
                "bool",
                "byte",
                "comparable",
                "complex64",
                "complex128",
                "error",
                "float32",
                "float64",
                "int",
                "int8",
                "int16",
                "int32",
                "int64",
                "rune",
                "string",
                "uint",
                "uint8",
                "uint16",
                "uint32",
                "uint64",
                "uintptr",
            ]
            .into_iter()
            .map(|name| (name.to_string(), GoType::Unknown))
            .collect(),
            provided: [
                "Append", "Copy", "Clear", "Close", "complex", "real", "imag", "delete", "len",
                "cap", "make", "min", "max", "new", "panic", "recover", "print",
                "println", //functions
                "true", "false", "iota", // Constants:
                "nil",  // Zero value:
            ]
            .into_iter()
            .map(|s| (s.to_string(), GoType::Unknown))
            .collect(),
            sub_scopes: vec![],
        }
    }

    pub fn add_variable(&mut self, provided: String, typ: GoType) {
        self.provided.insert(provided, typ);
    }

    pub fn add_provideds<I: IntoIterator<Item = (String, GoType)>>(&mut self, provided: I) {
        self.provided.extend(provided);
    }

    pub fn add_type(&mut self, name: String, typ: impl Into<GoType>) {
        self.types.insert(name, typ.into());
    }

    pub fn add_types<I: IntoIterator<Item = (String, GoType)>>(&mut self, types: I) {
        self.types.extend(types);
    }

    pub fn add_sub_scope(&mut self, sub_scope: Scope) {
        self.sub_scopes.push(sub_scope);
    }

    pub fn add_sub_scopes(&mut self, sub_scopes: Vec<Scope>) {
        self.sub_scopes.extend(sub_scopes);
    }
}

pub struct Context<'a> {
    scope: &'a mut Scope,
    pos: usize,
    parent_variables: Vec<&'a HashMap<String, GoType>>,
    parent_types: Vec<&'a HashMap<String, GoType>>,
}

impl<'a> Context<'a> {
    pub fn new(scope: &'a mut Scope) -> Self {
        Self {
            scope,
            pos: 0,
            parent_variables: vec![],
            parent_types: vec![],
        }
    }

    pub fn next_scope(&'a mut self) -> Option<Self> {
        let Self {
            scope,
            pos,
            parent_variables,
            parent_types,
        } = self;
        let sub_scope = scope.sub_scopes.get_mut(*pos)?;
        *pos += 1;
        let mut new_parent_variables = vec![&scope.provided];
        for variables in parent_variables.iter() {
            new_parent_variables.push(variables)
        }

        let mut new_parent_types = vec![&scope.types];
        for types in parent_types.iter() {
            new_parent_types.push(types)
        }

        Some(Self {
            pos: 0,
            scope: sub_scope,
            parent_variables: new_parent_variables,
            parent_types: new_parent_types,
        })
    }
}

impl<'a> std::ops::DerefMut for Context<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scope
    }
}

impl<'a> std::ops::Deref for Context<'a> {
    type Target = Scope;

    fn deref(&self) -> &Self::Target {
        &*self.scope
    }
}

#[derive(Clone, Debug)]
pub enum GoType {
    Bool,
    String,
    Int,
    Float,
    Function(FuncType),
    Declared(Type),
    Unknown,
    TypeParam(TypeElem),
}

impl From<Type> for GoType {
    fn from(value: Type) -> Self {
        Self::Declared(value)
    }
}

#[cfg(test)]
pub mod test {
    use std::assert_matches;

    use gosyn::Parser;

    use crate::{
        ast::{NameType, Type},
        name_resolution::{GoType, find_use_idents},
    };

    use super::Scope;

    fn get_scope_for(src: &'static str) -> Scope {
        let mut parser = Parser::from(src);
        let file = parser
            .parse_file()
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap()
            .into();

        find_use_idents(&file)
    }

    #[test]
    fn empty_file_produces_universe_with_one_empty_subscope() {
        let scope = get_scope_for("package main");
        assert_eq!(scope.sub_scopes.len(), 1);
        assert_eq!(scope.sub_scopes[0].types.len(), 0);
        assert_eq!(scope.sub_scopes[0].provided.len(), 0);
        assert_eq!(scope.sub_scopes[0].sub_scopes.len(), 0);
    }

    #[test]
    fn file_with_one_type_decl() {
        let scope = get_scope_for("package main\ntype id int");
        assert_eq!(scope.sub_scopes.len(), 1);
        assert_eq!(scope.sub_scopes[0].types.len(), 1);
        assert_matches!(
            scope.sub_scopes[0].types.get("id"),
            Some(GoType::Declared(Type::Name(NameType::Ident(name)))) if name.name.as_str()=="int"
        );
        assert_eq!(scope.sub_scopes[0].provided.len(), 0);
        assert_eq!(scope.sub_scopes[0].sub_scopes.len(), 0);
    }

    #[test]
    fn file_with_two_type_decls() {
        let scope = get_scope_for("package main\ntype id int\ntype test string");
        assert_eq!(scope.sub_scopes.len(), 1);
        assert_eq!(scope.sub_scopes[0].types.len(), 2);
        assert_matches!(
            scope.sub_scopes[0].types.get("id"),
            Some(GoType::Declared(Type::Name(NameType::Ident(name)))) if name.name.as_str()=="int"
        );

        assert_matches!(
            scope.sub_scopes[0].types.get("test"),
            Some(GoType::Declared(Type::Name(NameType::Ident(name)))) if name.name.as_str()=="string"
        );
        assert_eq!(scope.sub_scopes[0].provided.len(), 0);
        assert_eq!(scope.sub_scopes[0].sub_scopes.len(), 0);
    }

    #[test]
    fn file_with_function_type_decl_with_two_arguments() {
        let scope = get_scope_for("package main\ntype fun func(first, last name) int");
        assert_eq!(scope.sub_scopes.len(), 1);
        assert_eq!(scope.sub_scopes[0].types.len(), 1);
        assert_matches!(
            scope.sub_scopes[0].types.get("fun"),
            Some(GoType::Declared(Type::Function(_)))
        );

        assert_eq!(scope.sub_scopes[0].provided.len(), 0);
        assert_eq!(scope.sub_scopes[0].sub_scopes.len(), 1);

        assert_eq!(scope.sub_scopes[0].sub_scopes[0].types.len(), 0);
        // the function type has 2 arguments and no names in the return type.
        assert_eq!(scope.sub_scopes[0].sub_scopes[0].provided.len(), 2);
        assert_eq!(scope.sub_scopes[0].sub_scopes[0].sub_scopes.len(), 0);
    }

    #[test]
    fn file_with_function_type_decl_with_named_result() {
        let scope = get_scope_for("package main\ntype fun func(name) (age int)");
        assert_eq!(scope.sub_scopes.len(), 1);
        assert_eq!(scope.sub_scopes[0].types.len(), 1);
        assert_matches!(
            scope.sub_scopes[0].types.get("fun"),
            Some(GoType::Declared(Type::Function(_)))
        );

        assert_eq!(scope.sub_scopes[0].provided.len(), 0);
        assert_eq!(scope.sub_scopes[0].sub_scopes.len(), 1);

        assert_eq!(scope.sub_scopes[0].sub_scopes[0].types.len(), 0);
        // the function type does not have named arguments but has one named result.
        assert_eq!(scope.sub_scopes[0].sub_scopes[0].provided.len(), 1);
        assert_eq!(scope.sub_scopes[0].sub_scopes[0].sub_scopes.len(), 0);
    }

    #[test]
    fn file_with_function_decl_with_named_arguments_and_named_result() {
        let scope = get_scope_for("package main\nfunc test (first, last name) (age int)");
        assert_eq!(scope.sub_scopes.len(), 1);
        assert_eq!(scope.sub_scopes[0].types.len(), 0);
        assert_eq!(scope.sub_scopes[0].provided.len(), 1);
        assert_matches!(
            scope.sub_scopes[0].provided.get("test"),
            Some(GoType::Function(_))
        );

        assert_eq!(scope.sub_scopes[0].sub_scopes.len(), 1);

        assert_eq!(scope.sub_scopes[0].sub_scopes[0].types.len(), 0);
        // 2 named params + 1 named result
        assert_eq!(scope.sub_scopes[0].sub_scopes[0].provided.len(), 3);
        assert_eq!(scope.sub_scopes[0].sub_scopes[0].sub_scopes.len(), 0);
    }

    #[test]
    fn file_with_function_decl_with_type_params() {
        let scope = get_scope_for("package main\nfunc test[a, b any](first a, last b) (int){}");
        assert_eq!(scope.sub_scopes.len(), 1);
        assert_eq!(scope.sub_scopes[0].types.len(), 0);
        assert_eq!(scope.sub_scopes[0].provided.len(), 1);
        assert_matches!(
            scope.sub_scopes[0].provided.get("test"),
            Some(GoType::Function(_))
        );

        assert_eq!(scope.sub_scopes[0].sub_scopes.len(), 1);

        assert_eq!(scope.sub_scopes[0].sub_scopes[0].types.len(), 2);
        assert_matches!(
            scope.sub_scopes[0].sub_scopes[0].types["a"],
            GoType::TypeParam(_)
        );
        assert_matches!(
            scope.sub_scopes[0].sub_scopes[0].types["b"],
            GoType::TypeParam(_)
        );

        assert_eq!(scope.sub_scopes[0].sub_scopes[0].provided.len(), 2);
        assert_matches!(
            &scope.sub_scopes[0].sub_scopes[0].provided["first"],
            GoType::Declared(Type::Name(NameType::Ident(name))) if name.name.as_str() == "a"
        );
        assert_matches!(
            &scope.sub_scopes[0].sub_scopes[0].provided["last"],
            GoType::Declared(Type::Name(NameType::Ident(name))) if name.name.as_str() == "b"
        );

        assert_eq!(scope.sub_scopes[0].sub_scopes[0].sub_scopes.len(), 0);
    }
}
