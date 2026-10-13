use std::collections::{HashMap, hash_map::Entry};
use std::sync::atomic::{AtomicUsize, Ordering};

use gosyn::ast::InterfaceType;
use gosyn::{
    ast::{
        BlockStmt, Call, Declaration, Expression, File, Import, InstantiatedType, Statement,
        StructType,
    },
    token::LitKind,
};

fn main() {
    let mut args = std::env::args();
    let Some(file_path) = args.nth(1) else {
        eprintln!("No file specified");
        return;
    };
    let ast = match gosyn::parse_file(file_path) {
        Ok(ast) => ast,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };

    println!("{:?}", ast);
}

#[derive(Default)]
pub struct Context {
    pub types: HashMap<String, gosyn::ast::Type>,
    pub variables: HashMap<String, GoValue>,
}

pub fn convert_ast(file: File, context: &mut Context) -> Result<String, String> {
    let mut context = Context::default();
    let imports = file
        .imports
        .iter()
        .map(|import| convert_import(import, &mut context))
        .try_fold(String::new(), |a, b| -> Result<String, String> {
            Ok(a + &b?)
        })?;

    let declarations = file
        .decl
        .iter()
        .map(|decl| convert_decl(decl, &mut context))
        .try_fold(String::new(), |a, b| -> Result<String, String> {
            Ok(a + &b?)
        })?;

    return Ok(imports + &declarations);
}

fn convert_import(import: &Import, _context: &mut Context) -> Result<String, String> {
    let name = (import.name.as_ref().map(|i| i.name.as_str()))
        .or_else(|| import.path.value.split('/').last());
    if let Some(name) = name {
        return Ok(format!("use {name}::*;"));
    } else {
        return Err(format!("Not a valid package {}", import.path.value));
    }
}

fn convert_decl(decl: &Declaration, context: &mut Context) -> Result<String, String> {
    match decl {
        Declaration::Type(type_d) => {
            let mut types = String::new();
            for spec in &type_d.specs {
                context
                    .types
                    .insert(spec.name.name.clone(), spec.typ.clone());
                types += &convert_unnamed_type(&spec.typ).typ;
            }
            Ok(types)
        }
        Declaration::Function(func_decl) => todo!(),
        Declaration::Const(decl) => todo!(),
        Declaration::Variable(decl) => todo!(),
    }
}

pub fn resolve_ident<'a>(name: &'a str, context: &'a mut Context) -> Entry<'a, String, GoValue> {
    context.variables.entry(name.to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub enum GoValue {
    Int(String),
    Float32(String),
    String(String),
    Slice(Vec<GoValue>),
    Other,
}

fn find_use_idents(ast: &File) -> Scope {
    // TODO Have this be an over arching scope for all files and have proper package scopes.
    let mut scope = Scope::new_universe();

    for decl in &ast.decl {
        match decl {
            Declaration::Const(constants) => {
                for constant in &constants.specs {
                    for name in &constant.name {
                        scope.provided.push(name.name.clone())
                    }

                    if let Some(typ) = &constant.typ {
                        scope.add_containeds(find_idents_in_type(typ));
                    }

                    for value in &constant.values {
                        scope.add_containeds(find_idents_in_expression(&value));
                    }
                }
            }
            Declaration::Variable(variables) => {
                for variables in &variables.specs {
                    for name in &variables.name {
                        scope.add_provided(name.name.clone())
                    }

                    if let Some(typ) = &variables.typ {
                        scope.add_containeds(find_idents_in_type(typ));
                    }

                    for value in &variables.values {
                        scope.add_containeds(find_idents_in_expression(value));
                    }
                }
            }
            Declaration::Function(func_decl) => {
                scope.add_provided(func_decl.name.name.clone());
                let mut sub_scope = Scope::default();
                // For if this function is a method
                // TODO Check if recv can actually have more then one value

                if let Some(body) = &func_decl.body {
                    find_idents_in_block(body, &mut sub_scope)
                }
                scope.add_contained(sub_scope);
            }
            Declaration::Type(decl) => {
                for typ in &decl.specs {
                    scope.add_contained(typ.name.name.clone());
                    let mut sub_scope = Scope::default();
                    sub_scope.add_containeds(find_idents_in_type(&typ.typ));
                    scope.add_contained(sub_scope);
                }
            }
        }
    }
    scope
}

fn find_idents_in_statement(statement: &Statement, scope: &mut Scope) {
    match statement {
        Statement::Go(go) => scope.add_containeds(find_idents_in_call(&go.call)),
        Statement::If(if_stmt) => {
            let mut init_scope = Scope::default();
            if let Some(init) = &if_stmt.init {
                find_idents_in_statement(init, &mut init_scope);
            }
            init_scope.add_containeds(find_idents_in_expression(&if_stmt.cond));
            let mut if_scope = Scope::default();
            find_idents_in_block(&if_stmt.body, &mut if_scope);
            init_scope.add_contained(if_scope);
            if let Some(else_stmt) = &if_stmt.else_ {
                let mut else_scope = Scope::default();
                find_idents_in_statement(else_stmt, &mut else_scope);
                init_scope.add_contained(else_scope)
            }
            scope.add_contained(init_scope)
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
    }
}

fn find_idents_in_block(block: &BlockStmt, scope: &mut Scope) {
    for statement in &block.list {
        find_idents_in_statement(statement, scope);
    }
}

fn find_idents_in_expression(exp: &Expression) -> Vec<Contained> {
    use Contained::*;
    match exp {
        Expression::Ident(ident) => vec![Referenced(ident.name.clone())],
        Expression::BasicLit(lit) => match lit.kind {
            LitKind::Ident => vec![Referenced(lit.value.clone())],
            _ => vec![],
        },
        Expression::Index(index) => {
            let left = find_idents_in_expression(&index.left);
            let right = find_idents_in_expression(&index.index);
            [left, right].concat()
        }
        Expression::Operation(op) => {
            let left = find_idents_in_expression(&op.x);
            if let Some(right) = &op.y {
                let right = find_idents_in_expression(&right);
                [left, right].concat()
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
            idents.push(Referenced(selector.sel.name.clone()));
            idents
        }
        Expression::Range(range_expr) => find_idents_in_expression(&range_expr.right),
        Expression::Star(star_expression) => find_idents_in_expression(&star_expression.right),
        Expression::Paren(paren_expression) => find_idents_in_expression(&paren_expression.expr),
        Expression::TypeAssert(type_assertion) => {
            let left = find_idents_in_expression(&type_assertion.left);
            if let Some(right) = &type_assertion.right {
                let right = find_idents_in_type(&right);
                [left, right].concat()
            } else {
                left
            }
        }
        Expression::CompositeLit(composite_lit) => todo!(),
        Expression::Type(typ) => find_idents_in_type(typ),
    }
}

fn find_idents_in_type(typ: &gosyn::ast::Type) -> Vec<Contained> {
    use gosyn::ast::Type;
    match typ {
        Type::Map(map_type) => todo!(),
        Type::Array(array_type) => todo!(),
        Type::Slice(slice_type) => todo!(),
        Type::Function(func_type) => todo!(),
        Type::Struct(struct_type) => todo!(),
        Type::Channel(channel_type) => todo!(),
        Type::Pointer(pointer_type) => todo!(),
        Type::Interface(interface_type) => todo!(),
        Type::Name(name_type) => todo!(),
        Type::Instantiated(instantiated_type) => todo!(),
        Type::Paren(typ) => find_idents_in_type(&typ.typ),
    }
}

pub fn find_idents_in_call(call: &Call) -> Vec<Contained> {
    let mut idents = find_idents_in_expression(&call.func);
    for arg in &call.args {
        idents.extend(find_idents_in_expression(&arg));
    }
    idents
}

/// Contains lists of variables declared and used in a given program scope such as the entire program or a function.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// Named generics, function arguements or variables declared in this scope
    provided: Vec<String>,
    /// All identifiers that are refered to in this scope
    referenced: Vec<String>,
    /// Functions declared in this scope with their own arguements and local variables
    sub_scopes: Vec<Scope>,
}

impl Scope {
    pub fn new(provided: Vec<String>, referenced: Vec<String>, sub_scopes: Vec<Scope>) -> Self {
        Self {
            provided,
            referenced,
            sub_scopes,
        }
    }

    pub fn new_universe() -> Self {
        Self {
            provided: vec![
                // Types:
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
                // Constants:
                "true",
                "false",
                "iota",
                // Zero value:
                "nil",
                // Functions:
                "append",
                "cap",
                "clear",
                "close",
                "complex",
                "copy",
                "delete",
                "imag",
                "len",
                "make",
                "max",
                "min",
                "new",
                "panic",
                "print",
                "println",
                "real",
                "recover",
            ]
            .into_iter()
            .map(|x| x.to_string())
            .collect(),
            referenced: vec![],
            sub_scopes: vec![],
        }
    }

    pub fn add_provided(&mut self, provided: String) {
        self.provided.push(provided);
    }

    pub fn add_provideds<I: IntoIterator<Item = String>>(&mut self, provided: I) {
        self.provided.extend(provided);
    }

    pub fn add_referenced(&mut self, referenced: String) {
        self.referenced.push(referenced);
    }

    pub fn add_references<I: IntoIterator<Item = String>>(&mut self, references: I) {
        self.referenced.extend(references);
    }

    pub fn add_sub_scope(&mut self, sub_scope: Scope) {
        self.sub_scopes.push(sub_scope);
    }

    pub fn add_sub_scopes(&mut self, sub_scopes: Vec<Scope>) {
        self.sub_scopes.extend(sub_scopes);
    }

    pub fn add_contained<C: Into<Contained>>(&mut self, contained: C) {
        let contained = contained.into();
        match contained {
            Contained::Referenced(reference) => self.referenced.push(reference),
            Contained::SubScope(sub_scope) => self.sub_scopes.push(sub_scope),
        }
    }

    pub fn add_containeds<C: Into<Contained>, I: IntoIterator<Item = C>>(&mut self, contained: I) {
        for contained in contained {
            let contained = contained.into();
            match contained {
                Contained::Referenced(reference) => self.referenced.push(reference),
                Contained::SubScope(sub_scope) => self.sub_scopes.push(sub_scope),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Contained {
    Referenced(String),
    SubScope(Scope),
}

impl From<String> for Contained {
    fn from(name: String) -> Self {
        Self::Referenced(name)
    }
}

impl From<Scope> for Contained {
    fn from(scope: Scope) -> Self {
        Self::SubScope(scope)
    }
}

pub struct WithExtraTypes {
    typ: String,
    extras: Vec<String>,
}

fn convert_unnamed_type(typ: &gosyn::ast::Type) -> WithExtraTypes {
    use gosyn::ast::Type;
    match typ {
        Type::Name(name) => WithExtraTypes {
            typ: convert_name(name),
            extras: vec![],
        },
        Type::Instantiated(InstantiatedType {
            name, arguements, ..
        }) => {
            let mut extras = vec![];
            let typ = convert_name(name);
            let arguments = arguements
                .iter()
                .map(convert_unnamed_type)
                .map(|arg| {
                    extras.extend(arg.extras);
                    arg.typ
                })
                .reduce(|a, b| format!("{a}, {b}"));
            let typ = match arguments {
                Some(arguements) => format!("{typ}<{arguements}>"),
                None => typ,
            };
            WithExtraTypes { typ, extras }
        }
        Type::Struct(struct_t) => {
            let name = generate_type_name(AnonymousType::Struct(&struct_t));
            convert_struct(&name, struct_t)
        }
        _ => unimplemented![],
    }
}

fn convert_name(name: &gosyn::ast::NameType) -> String {
    match name {
        gosyn::ast::NameType::Ident(ident) => ident.name.clone(),
        gosyn::ast::NameType::Qualified(qual_ident) => {
            format!(
                "{}::{}",
                qual_ident.package.name, qual_ident.identifier.name
            )
        }
    }
}

pub enum AnonymousType<'a> {
    Struct(&'a StructType),
    Interface(&'a InterfaceType),
}

fn generate_type_name(typ: AnonymousType) -> String {
    let count = next_anon_count();
    match typ {
        AnonymousType::Struct(_) => format!("AnonymousStruct{count}"),
        AnonymousType::Interface(_) => format!("AnonymousInterface{count}"),
    }
}

static ANON_TYPE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_anon_count() -> usize {
    ANON_TYPE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1
}

fn convert_struct(name: &str, typ: &StructType) -> WithExtraTypes {
    let mut extras = vec![];
    let header = format!("struct {name}{{");
    let mut fields = vec![];
    for field in &typ.fields {
        let WithExtraTypes {
            typ,
            extras: field_extras,
        } = convert_unnamed_type(&field.typ);
        extras.extend(field_extras);
        if field.name.len() == 0 {
            fields.push(format!("{typ}: {typ},"));
        } else {
            for name in &field.name {
                fields.push(format!("{}: {},", &name.name, typ));
            }
        }
    }
    let fields = fields.join("\n");
    let tail = "}";
    let typ = format!("{header}\n{fields}\n{tail}");
    WithExtraTypes { typ, extras }
}
