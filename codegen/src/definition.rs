use std::collections::HashMap;
use crate::types::{Field, FieldMap, FieldType};
use crate::Node;

pub struct CodeDefinition<'a> {
    pub definition_type: CodeDefinitionType,
    pub context: CodeContext<'a>,
}

pub enum CodeDefinitionType {
    Struct,
    Func, //Later on, i'll add a namespace here.
}
impl<'a> CodeDefinition<'a> {
    
}

pub struct CodeContext<'a> {
    pub parent_context: Option<&'a CodeContext<'a>>,
    pub depth: u32,
    pub definition_map: HashMap<String, CodeDefinition<'a>>,
    pub body: Vec<Node>,
}

impl<'a> CodeContext<'a> {
    pub fn new(parent_context: &'a CodeContext<'a>) -> CodeContext<'a> {
        Self{
            parent_context: Some(parent_context),
            depth: parent_context.depth + 1,
            definition_map: HashMap::new(),
            body: Vec::new(),
        }
    }
}