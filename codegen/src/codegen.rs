use dfbin::DFBin;
use parser::parser::Node;
use crate::errors::CodegenError;

pub struct CodeGen {
    pub token_tree: Vec<Node>,
    pub buffer: DFBin,
}

impl CodeGen {
    pub fn new() -> CodeGen {
        Self {
            token_tree: Vec::new(),
            buffer: DFBin::new()
        }
    }

    pub fn from_tokens(tokens: Vec<Node>) -> CodeGen {
        Self {
            token_tree: tokens,
            buffer: DFBin::new()
        }
    }

    pub fn generate() -> Result<(), CodegenError> {
        
        Ok(())
    }
}