use std::{fmt, rc::Rc};

use parser::parser::Node;

#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Compiler error.")]
pub struct CodegenError {
    token: Option<ErrorToken>,
    pub source: ErrorRepr,
}

#[derive(Debug, PartialEq)]
pub struct ErrorToken {
    pub token: Rc<Node>,
    pub position: usize
}
impl fmt::Display for ErrorToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}; {:?})", self.position, self.token)
    }
}

impl CodegenError {
    pub fn new(node: Rc<Node>, source: ErrorRepr) -> CodegenError {
        Self {
            token: Some(ErrorToken {
                token: node,
                position: 0
            }),
            source
        }
    }
    pub fn err<T>(node: Rc<Node>, source: ErrorRepr) -> Result<T, CodegenError> {
        Err(Self::new(node, source))
    }
    pub fn new_position(node: Rc<Node>, position: usize, source: ErrorRepr) -> CodegenError {
        Self {
            token: Some(ErrorToken {
                token: node,
                position
            }),
            source
        }
    }
    pub fn new_headless(source: ErrorRepr) -> CodegenError {
        Self {
            token: None,
            source
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ErrorRepr {
    #[error("Generic Error")]
    Generic,
    #[error("Expected a code block.")]
    ExpectedBlock,
    #[error("Expected a scannable code block.")]
    ExpectedScannableBlock,
}
