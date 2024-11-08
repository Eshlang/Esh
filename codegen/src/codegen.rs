use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dfbin::DFBin;
use lexer::types::TokenType;
use parser::parser::Node;
use crate::errors::{CodegenError, ErrorRepr};
use crate::context::{CodeDefinition, CodeScope, Context, ContextType};
use crate::types::{Field, FieldModifier, FieldType, PrimitiveType};

pub struct CodeGen {
    pub context_map: HashMap<String, usize>,
    pub root_context: usize,
    pub contexts: Vec<Rc<RefCell<Context>>>,
    pub current_id: usize,
    pub buffer: DFBin,
    pub run: usize,
}

impl CodeGen {
    pub fn new() -> CodeGen {
        Self {
            context_map: HashMap::new(),
            root_context: 0,
            contexts: Vec::new(),
            current_id: 0,
            buffer: DFBin::new(),
            run: 0
        }
    }

    fn scan_block_outline(&mut self, node_block: Rc<Node>, context_type: ContextType, mut depth: u32, parent_id: usize, scope: CodeScope) -> Result<usize, CodegenError> {
        self.run += 1;
        let Node::Block(node_block) = node_block.as_ref() else {
            return CodegenError::err(node_block, ErrorRepr::ExpectedBlock);
        };
        let current_id = self.current_id;
        let current_context_cell = Rc::new(RefCell::new(Context::new_empty(context_type.clone(), parent_id, current_id, depth, Vec::new(), scope)));
        depth += 1;
        self.contexts.push(current_context_cell.clone());
        let mut current_context = current_context_cell.borrow_mut();
        self.current_id += 1;
        for node in node_block {
            match (node.as_ref(), &context_type) {
                (Node::Func(ident, params, return_type, body), _) => {
                    let return_type_field = FieldType::Ident(return_type.clone());
                    let child_id = self.scan_block_outline(body.clone(), ContextType::Function(return_type_field), depth, current_id, CodeScope::Public)?;
                    let Node::Primary(TokenType::Ident(ident_string)) = ident.as_ref() else {
                        return CodegenError::err(ident.clone(), ErrorRepr::ExpectedFunctionIdentifier)
                    };

                    let mut child_modify = self.context_borrow_mut(child_id)?;
                    let params = Self::extract_declaration_vec(params)?;
                    for (param_type, param_name) in params {
                        let Node::Primary(TokenType::Ident(param_name_ident)) = param_name.as_ref() else {
                            return CodegenError::err(param_name.clone(), ErrorRepr::ExpectedFunctionParamIdent)
                        };
                        let field_id = child_modify.fields.len();
                        child_modify.fields.push(Field {
                            field_type: FieldType::Ident(param_type.clone()),
                            modifier: FieldModifier::None,
                            scope: CodeScope::Public,
                        });
                        Self::add_definition(&mut child_modify, param_name_ident.clone(), CodeDefinition::Field(field_id))?;
                    }
                    drop(child_modify);

                    Self::add_definition(&mut current_context, ident_string.clone(), CodeDefinition::Context(child_id))?;
                    current_context.children.push(child_id);
                },
                (Node::Struct(ident, body), _) => {
                    let child_id = self.scan_block_outline(body.clone(), ContextType::Struct, depth, current_id, CodeScope::Public)?;
                    let Node::Primary(TokenType::Ident(ident_string)) = ident.as_ref() else {
                        return CodegenError::err(ident.clone(), ErrorRepr::ExpectedStructIdentifier)
                    };
                    Self::add_definition(&mut current_context, ident_string.clone(), CodeDefinition::Context(child_id))?;
                    current_context.children.push(child_id);
                },
                (Node::Declaration(field_type, field_name), ContextType::Struct) => {
                    let Node::Primary(TokenType::Ident(field_name_ident)) = field_name.as_ref() else {
                        return CodegenError::err(field_name.clone(), ErrorRepr::ExpectedStructIdentifier)
                    };
                    let field_id = current_context.fields.len();
                    Self::add_definition(&mut current_context, field_name_ident.clone(), CodeDefinition::Field(field_id))?;
                    current_context.fields.push(Field{
                        field_type: FieldType::Ident(field_type.clone()),
                        modifier: FieldModifier::None,
                        scope: CodeScope::Public,
                    })
                }
                (_, ContextType::Struct) => {
                    return CodegenError::err(node.clone(), ErrorRepr::UnstructuredStructCode);
                }
                _ => {
                    current_context.body.push(node.clone());
                }
            };
        };
        
        drop(current_context);
        Ok(current_id)
    }

    fn add_definition(context: &mut Context, ident: String, definition: CodeDefinition) -> Result<(), CodegenError> {
        match context.definition_lookup.get_mut(&ident) {
            Some(CodeDefinition::Multiple(mult)) => {
                mult.push(definition);
            },
            Some(existing_definition) => {
                let mut mult_vec = Vec::new();
                mult_vec.push(existing_definition.clone());
                mult_vec.push(definition);
                context.definition_lookup.insert(ident, CodeDefinition::Multiple(mult_vec));
            },
            None => {
                context.definition_lookup.insert(ident, definition);
            },
        }
        Ok(())
    }

    fn extract_declaration_vec(node: &Rc<Node>) -> Result<Vec<(&Rc<Node>, &Rc<Node>)>, CodegenError> {
        match node.as_ref() {
            Node::None => Ok(Vec::new()),
            Node::Declaration(node_type, node_ident) => Ok(vec![(node_type, node_ident)]),
            Node::Tuple(node_children) => {
                let mut declarations = Vec::new();
                for node_child in node_children {
                    let Node::Declaration(node_child_type, node_child_ident) = node_child.as_ref() else {
                        return CodegenError::err(node_child.clone(), ErrorRepr::ExpectedFunctionParamDeclaration);
                    };
                    declarations.push((node_child_type, node_child_ident));
                }
                Ok(declarations)
            }
            _ => CodegenError::err(node.clone(), ErrorRepr::ExpectedFunctionParamDeclaration)
        }
    }
    
    
    fn fill_all_field_types(&mut self) -> Result<(), CodegenError> {
        let mut context_id = 0;
        for context in self.contexts.iter() {
            let mut context_modify = CodegenError::map_headless(context.try_borrow_mut(), ErrorRepr::BadBorrow)?;
            if let ContextType::Function(func_return_type) = std::borrow::BorrowMut::borrow_mut(&mut context_modify.context_type) {
                if let FieldType::Ident(func_return_type_node) = func_return_type {
                    if !matches!(func_return_type_node.as_ref(), Node::None) {
                        self.fill_field_type(func_return_type, context_id)?;
                    }
                };
            }
            for field in context_modify.fields.iter_mut() {
                self.fill_field_type(&mut field.field_type, context_id)?;
            }
            drop(context_modify);
            context_id += 1;
        }
        Ok(())
    }

    fn fill_field_type(&self, field: &mut FieldType, context: usize) -> Result<(), CodegenError> {
        let FieldType::Ident(field_ident) = field else {
            return Ok(());
        };
        *field = self.find_type_by_ident(field_ident, context)?;
        Ok(())
    }

    fn context_borrow(&self, context: usize) -> Result<std::cell::Ref<'_, Context>, CodegenError> {
        println!("[!] Borrowing {}", context);
        let g = CodegenError::map_headless(self.contexts[context].try_borrow(), ErrorRepr::BadBorrow);
        if g.is_err() {
            panic!();
        }
        g
    }

    fn context_borrow_mut(&self, context: usize) -> Result<std::cell::RefMut<'_, Context>, CodegenError> {
        println!("[!] Mutably Borrowing {}", context);
        let g = CodegenError::map_headless(self.contexts[context].try_borrow_mut(), ErrorRepr::BadBorrow);
        if g.is_err() {
            panic!();
        }
        g
    }

    fn find_type_by_ident(&self, ident: &Rc<Node>, context: usize) -> Result<FieldType, CodegenError> {
        let Node::Primary(TokenType::Ident(ident_string)) = ident.as_ref() else {
            println!("{:?}", ident);
            return CodegenError::err(ident.clone(), ErrorRepr::ExpectedTypeIdent)
        };
        match Self::is_definition_primitive(ident_string) {
            Some(primitive) => Ok(FieldType::Primitive(primitive)),
            None => {
                let definition = self.find_definition_by_ident(ident, context)?;
                let definition = Self::extract_definition_struct(definition)?;
                Ok(FieldType::Struct(definition))
            }
        }
    }

    fn is_definition_primitive(ident_string: &str) -> Option<PrimitiveType> {
        match ident_string {
            "num" => Some(PrimitiveType::Number),
            "string" => Some(PrimitiveType::String),
            _ => None
        }
    }

    fn extract_definition_struct(definition: CodeDefinition) -> Result<usize, CodegenError> {
        match definition {
            CodeDefinition::Context(context) => Ok(context),
            CodeDefinition::Multiple(definitions) => {
                let mut result = CodegenError::err_headless(ErrorRepr::ExpectedStruct);
                for check_definition in definitions {
                    if let CodeDefinition::Context(context) = check_definition {
                        result = Ok(context);
                        break;
                    }
                }
                result
            },
            _ => CodegenError::err_headless(ErrorRepr::ExpectedStruct)
        }
    }

    fn find_definition_by_ident(&self, ident: &Rc<Node>, mut context: usize) -> Result<CodeDefinition, CodegenError> {
        let Node::Primary(TokenType::Ident(ident_string)) = ident.as_ref() else {
            return CodegenError::err(ident.clone(), ErrorRepr::ExpectedTypeIdent)
        };
        loop {
            let context_borrow = self.context_borrow(context)?;
            match context_borrow.definition_lookup.get(ident_string) {
                Some(def) => {
                    return Ok(def.clone());
                }
                None => {
                    if context_borrow.id == context_borrow.parent_id {
                        return CodegenError::err(ident.clone(), ErrorRepr::TypeIdentNotRecognized)
                    }
                    context = context_borrow.parent_id;
                }
            };
            drop(context_borrow);
        }
    }


    pub fn generate_at_root(&mut self, node: Rc<Node>) -> Result<(), CodegenError> {
        let context = self.scan_block_outline(node, ContextType::Namespace, 0, 0, CodeScope::Public)?;
        println!("\nFilling in all field types:\n");
        self.fill_all_field_types()?;
        self.root_context = 0;
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
struct Player {
    string uuid;
    num hp;
    
    func damage(num dmg) {
        hp = hp - dmg;
    }
}
func test(num myNum, string p) {
    myNum = myNum - 10;
    num secondGuy = (myNum * 5) + 2;
    p = "Guy: " + myNum + ", Second Guy: " + secondGuy;
}
func hello(string hell, num add) -> string {
    hell = "crazy" + add;
    return hell;
}
func damagePlayer(Player player) -> Player {
    player.damage(5);
    return player;
}
"##);
        let lexer_tokens: Vec<types::Token> = lexer.map(|v| v.expect("Lexer token should unwrap")).collect();
        println!("LEXER TOKENS\n----------------------\n{:#?}\n----------------------", lexer_tokens);
        let mut parser = Parser::new(lexer_tokens.as_slice());
        let parser_tree = Rc::new(parser.statement_block().expect("Parser statement block should unwrap"));
        println!("PARSER TREE\n----------------------\n{:#?}\n----------------------", parser_tree);

        let mut codegen = CodeGen::new();
        codegen.generate_at_root(parser_tree.clone()).expect("Codegen should generate");
        println!("CODEGEN CONTEXTS\n----------------------\n{:#?}\n----------------------", codegen.contexts);
    
    
    }
}