use std::collections::HashMap;

use dfbin::DFBin;
use parser::parser::Node;
use crate::errors::CodegenError;
use crate::definition::CodeDefinition;

pub struct CodeGen {
    pub token_tree: Vec<Node>,
    pub general_pointer: usize,
    pub context: HashMap<String, CodeDefinition>,
    pub buffer: DFBin,
}

impl CodeGen {
    pub fn new() -> CodeGen {
        Self {
            token_tree: Vec::new(),
            general_pointer: 0,
            context: HashMap::new(),
            buffer: DFBin::new(),
        }
    }

    pub fn from_tokens(tokens: Vec<Node>) -> CodeGen {
        Self {
            token_tree: tokens,
            general_pointer: 0,
            context: HashMap::new(),
            buffer: DFBin::new(),
        }
    }

    pub fn scan(&mut self) -> Result<(), CodegenError> {
        self.general_pointer = 0;
        loop {
            self.scan_next()?;
        }
        Ok(())
    }

    fn scan_next(&mut self) -> Result<(), CodegenError> {
        Ok(())
    }


    pub fn generate(&mut self) -> Result<(), CodegenError> {
        self.general_pointer = 0;
        self.scan()?;


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

        let mut codegen = CodeGen::from_tokens(vec![g]);
        codegen.generate().expect("Codegen should generate.");
        
    }
}