use std::fmt;

use parser::parser::Node;

#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Compiler error.")]
pub struct CodegenError {
    token: Option<ErrorToken>,
    pub source: CodegenErrorKind,
}

#[derive(Debug, PartialEq)]
pub struct ErrorToken {
    pub token: Node,
    pub position: usize
}
impl fmt::Display for ErrorToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}; {:?})", self.position, self.token)
    }
}

impl CodegenError {
    pub fn new(node: Node, position: usize, source: CodegenErrorKind) -> CodegenError {
        Self {
            token: Some(ErrorToken {
                token: node,
                position
            }),
            source
        }
    }
    pub fn new_headless(source: CodegenErrorKind) -> CodegenError {
        Self {
            token: None,
            source
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum CodegenErrorKind {
    #[error("Generic Error")]
    Generic,
}
