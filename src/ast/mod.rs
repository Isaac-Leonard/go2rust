//! Golang AST nodes without positional information

mod declarations;
mod expressions;
mod statements;
mod types;

pub use declarations::*;
pub use expressions::*;
pub use statements::*;
pub use types::*;

use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;

use crate::utils::VecExt;

#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,
}

impl From<gosyn::ast::Comment> for Comment {
    fn from(value: gosyn::ast::Comment) -> Self {
        Self { text: value.text }
    }
}

impl From<Rc<gosyn::ast::Comment>> for Comment {
    fn from(value: Rc<gosyn::ast::Comment>) -> Self {
        Self {
            text: value.text.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ident {
    pub name: String,
}

impl Ident {
    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }
}

impl From<gosyn::ast::Ident> for Ident {
    fn from(value: gosyn::ast::Ident) -> Self {
        Self {
            name: value.name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QualifiedName {
    pub package: Ident,
    pub identifier: Ident,
}

impl From<gosyn::ast::QualifiedName> for QualifiedName {
    fn from(value: gosyn::ast::QualifiedName) -> Self {
        Self {
            package: value.package.into(),
            identifier: value.identifier.into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Import {
    pub name: Option<Ident>,
    pub path: StringLit,
}

impl From<gosyn::ast::Import> for Import {
    fn from(value: gosyn::ast::Import) -> Self {
        Self {
            name: value.name.map(Into::into),
            path: value.path.into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub line_info: Vec<usize>,
    pub docs: Vec<Comment>,
    pub pkg_name: Ident,
    pub imports: Vec<Import>,
    pub decl: Vec<Declaration>,
    pub comments: Vec<Comment>,
}

impl From<gosyn::ast::File> for File {
    fn from(value: gosyn::ast::File) -> Self {
        Self {
            path: value.path,
            line_info: value.line_info,
            docs: value.docs.map_into(),
            pkg_name: value.pkg_name.into(),
            imports: value.imports.map_into(),
            decl: value.decl.map_into(),
            comments: value.comments.map_into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub path: PathBuf,
    pub files: Vec<File>,
}
