use std::collections::HashMap;
use std::rc::Rc;
use crate::types::{Field, FieldMap, FieldType};
use crate::Node;

pub struct CodeDefinition {
    pub definition_type: CodeDefinitionType,
    pub context: Rc<CodeContext>,
}

pub enum CodeDefinitionType {
    Struct,
    Func, //Later on, i'll add a namespace here.
}
impl CodeDefinition{
    
}


pub struct CodeContext {
    pub parent_context: Option<Rc<CodeContext>>,
    pub depth: u32,
    pub definition_map: HashMap<String, CodeDefinition>,
    pub body: Vec<Rc<Node>>,
}

impl CodeContext {
    pub fn new(parent_context: Rc<CodeContext>, body: Vec<Rc<Node>>) -> CodeContext {
        Self {
            depth: parent_context.depth + 1,
            parent_context: Some(parent_context),
            definition_map: HashMap::new(),
            body
        }
    }
    pub fn with_map(parent_context: Rc<CodeContext>, body: Vec<Rc<Node>>, definitions: HashMap<String, CodeDefinition>) -> CodeContext {
        Self {
            depth: parent_context.depth + 1,
            parent_context: Some(parent_context),
            definition_map: definitions,
            body
        }
    }
}