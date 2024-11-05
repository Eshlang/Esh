use std::collections::HashMap;

use dfbin::DFBin;
use parser::parser::Node;
use crate::errors::CodegenError;
use crate::definition::{CodeDefinition, CodeContext};

pub struct CodeGen {
    pub context_map: HashMap<String, usize>,
    pub buffer: DFBin,
}

impl CodeGen {
    pub fn new() -> CodeGen {
        Self {
            context_map: HashMap::new(),
            buffer: DFBin::new(),
        }
    }

    fn scan_context(&mut self, node: &Node, parent_context: &CodeContext) -> Result<CodeContext, CodegenError> {
        let context = todo!();
        match node {
            _ => {}
        }
        Ok(todo!())
    }

    pub fn generate(&mut self, node: &Node, parent_context: &CodeContext) -> Result<(), CodegenError> {
        let context = self.scan_context(&node, parent_context)?;


        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use parser::parser::*;
    use lexer::{Lexer, types};
    use super::*;

    #[test]
    pub fn parse_string_test() {
        let lexer = Lexer::new(r##"
func test(num myNum, string p) {
    myNum = myNum - 10;
    num secondGuy = (myNum * 5) + 2;
    p = "Guy: " + myNum + ", Second Guy: " + secondGuy;
}
func hello(string hell, num add) -> string {
    hell = "crazy" + add;
    return hell;
}
"##);
        let lexer_tokens: Vec<types::Token> = lexer.map(|v| v.expect("Lexer token should unwrap")).collect();
        let mut parser = Parser::new(lexer_tokens.as_slice());
        let g = parser.statement_block().expect("Parser statement block should unwrap");
        println!("{:#?}", g);

        let mut codegen = CodeGen::new();
        // codegen.generate().expect("Codegen should generate.");
        
    }
}