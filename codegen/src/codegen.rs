use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use dfbin::DFBin;
use lexer::types::TokenType;
use parser::parser::Node;
use crate::errors::{CodegenError, ErrorRepr};
use crate::context::{CodeContext, CodeDefinition, CodeDefinitionType};

pub struct CodeGen {
    pub context_map: HashMap<String, usize>,
    pub buffer: DFBin,
    pub run: usize,
}

impl CodeGen {
    pub fn new() -> CodeGen {
        Self {
            context_map: HashMap::new(),
            buffer: DFBin::new(),
            run: 0
        }
    }

    fn scan_context(&mut self, node: Rc<Node>, parent_context: Rc<CodeContext>) -> Result<Rc<CodeContext>, CodegenError> {
        self.run += 1;
        let body = match node.as_ref() {
            Node::Block(_) => {
                Self::get_block_code(&node)?
            }
            Node::Func(_, _, _, func_block) => {
                Self::get_block_code(func_block)?
            }
            Node::Struct(_, struct_block) => {
                Self::get_block_code(struct_block)?
            }
            _ => return CodegenError::err(node, ErrorRepr::ExpectedScannableBlock)
        };
        let mut definitions = HashMap::new();
        for child_node in body.iter() {
            match child_node.as_ref() {
                Node::Func(func_ident, func_parameters, func_return_type, func_block) => {
                    // Self::get_block_code(func_block)?
                }
                Node::Struct(struct_ident, struct_block) => {
                    definitions.insert(
                        Self::get_ident_string(struct_ident)?,
                        CodeDefinition {
                            definition_type: CodeDefinitionType::Struct,
                            context: self.scan_context(child_node.clone(), node.clone())?
                        }
                    );
                }
                _ => {}
            }
        }
        let context = Rc::new(CodeContext::with_map(
            parent_context, 
            body,
            definitions
        ));
        Ok(context)
    }

    fn get_ident_string(node: &Rc<Node>) -> Result<String, CodegenError> {
        let Node::Primary(TokenType::Ident(val)) = node.as_ref() else {
            return CodegenError::err(node.to_owned(), ErrorRepr::ExpectedBlock);
        };
        Ok(val.to_owned())
    }

    fn get_block_code(node: &Rc<Node>) -> Result<Vec<Rc<Node>>, CodegenError> {
        let Node::Block(vec) = node.as_ref() else {
            return CodegenError::err(node.to_owned(), ErrorRepr::ExpectedBlock);
        };
        Ok(vec.to_owned())
    }


    pub fn generate(&mut self, node: Rc<Node>, parent_context: Rc<CodeContext>) -> Result<(), CodegenError> {
        let context = self.scan_context(node, parent_context)?;


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