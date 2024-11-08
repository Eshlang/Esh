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
                    let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedFunctionIdentifier)?;

                    let mut child_modify = self.context_borrow_mut(child_id)?;
                    let params = Self::extract_declaration_vec(params)?;
                    for (param_type, param_name) in params {
                        let param_name_ident = Self::get_primary_as_ident(param_name, ErrorRepr::ExpectedFunctionParamIdent)?;
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
                    let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedStructIdentifier)?;
                    Self::add_definition(&mut current_context, ident_string.clone(), CodeDefinition::Context(child_id))?;
                    current_context.children.push(child_id);
                },
                (Node::Declaration(field_type, field_name), ContextType::Struct) => {
                    let field_name_ident = Self::get_primary_as_ident(field_name, ErrorRepr::ExpectedStructFieldIdentifier)?;
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

    fn get_primary_as_ident(node: &Rc<Node>, err: ErrorRepr) -> Result<&String, CodegenError> {
        let Node::Primary(node_token) = node.as_ref() else {
            return CodegenError::err(node.clone(), err);
        };
        let TokenType::Ident(node_ident) = &node_token.token_type else {
            return CodegenError::err(node.clone(), err);
        };
        Ok(node_ident)
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
        for context_id in 0..self.contexts.len() {
            let context_get = self.context_borrow(context_id)?;
            let context_type = context_get.context_type.clone();
            let fields_len = context_get.fields.len();
            drop(context_get);

            if let ContextType::Function(func_return_type) = context_type {
                if let FieldType::Ident(func_return_type_node) = func_return_type {
                    if !matches!(func_return_type_node.as_ref(), Node::None) {
                        let return_type_set = self.find_type_by_ident(&func_return_type_node, context_id)?;
                        let mut context_get_mut = self.context_borrow_mut(context_id)?;
                        context_get_mut.context_type = ContextType::Function(return_type_set);
                        drop(context_get_mut);
                    }
                };
            }
            for field in 0..fields_len {
                let context_get = self.context_borrow(context_id)?;
                let FieldType::Ident(field_ident) = context_get.fields.get(field).unwrap().field_type.clone() else {
                    continue;
                };
                drop(context_get);
                let field_type_set =  self.find_type_by_ident(&field_ident, context_id)?;
                let mut context_get_mut = self.context_borrow_mut(context_id)?;
                context_get_mut.fields[field].field_type = field_type_set;
                drop(context_get_mut);
            }
            }
        Ok(())
    }

    fn context_borrow(&self, context: usize) -> Result<std::cell::Ref<'_, Context>, CodegenError> {
        CodegenError::map_headless(self.contexts[context].try_borrow(), ErrorRepr::BadBorrow)
    }

    fn context_borrow_mut(&self, context: usize) -> Result<std::cell::RefMut<'_, Context>, CodegenError> {
        CodegenError::map_headless(self.contexts[context].try_borrow_mut(), ErrorRepr::BadMutBorrow)
    }

    fn find_type_by_ident(&self, ident: &Rc<Node>, context: usize) -> Result<FieldType, CodegenError> {
        let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedTypeIdent)?;
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
        let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedTypeIdent)?;
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
    
    fn scan_all_bodies(&mut self) -> Result<(), CodegenError> {
        for context_id in 0..self.contexts.len() {
            self.scan_body(context_id)?;
        }
        Ok(())
    }

    fn scan_body(&mut self, context: usize) -> Result<(), CodegenError> {
        let context_modify = self.context_borrow(context)?;

        drop(context_modify);
        Ok(())
    }


    pub fn generate_at_root(&mut self, node: Rc<Node>) -> Result<(), CodegenError> {
        let _root_context = self.scan_block_outline(node, ContextType::Namespace, 0, 0, CodeScope::Public)?;
        self.fill_all_field_types()?;
        self.scan_all_bodies()?;
        self.root_context = 0;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use parser::parser::*;
    use lexer::{Lexer, types::Token};
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
func test2(num number1) {
    num number2;
    num number3 = 5;
}
"##);
        let lexer_tokens: Vec<Rc<Token>> = lexer.map(|v| Rc::new(v.expect("Lexer token should unwrap"))).collect();
        println!("LEXER TOKENS\n----------------------\n{:#?}\n----------------------", lexer_tokens);
        let mut parser = Parser::new(lexer_tokens.as_slice());
        let parser_tree = Rc::new(parser.parse().expect("Parser statement block should unwrap"));
        println!("PARSER TREE\n----------------------\n{:#?}\n----------------------", parser_tree);

        let mut codegen = CodeGen::new();
        codegen.generate_at_root(parser_tree.clone()).expect("Codegen should generate");
        println!("CODEGEN CONTEXTS\n----------------------\n{:#?}\n----------------------", codegen.contexts);
    
    
    }
}