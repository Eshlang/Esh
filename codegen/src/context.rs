use std::collections::HashMap;
use std::rc::Rc;
use crate::types::{Field, FieldType};
use crate::Node;

#[derive(Clone, Debug)]
pub enum CodeDefinition {
    Context(usize),
    Field(usize),
    Multiple(Vec<CodeDefinition>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContextType {
    Struct,
    Function(FieldType),
    Namespace,
}

#[derive(Debug, Clone)]
pub enum CodeScope {
    Public,
    Private
}

#[derive(Debug)]
pub struct Context {
    pub context_type: ContextType,
    pub parent_id: usize,
    pub id: usize,
    pub depth: u32,
    pub fields: Vec<Field>,
    pub definition_lookup: HashMap<String, CodeDefinition>,
    pub body: Rc<Vec<Rc<Node>>>,
    pub scope: CodeScope,
    pub children: Vec<usize>,
}

impl Context {
    pub fn new_empty(context_type: ContextType, parent_id: usize, id: usize, depth: u32, body: Rc<Vec<Rc<Node>>>, scope: CodeScope) -> Context {
        Self {
            context_type,
            parent_id,
            children: Vec::new(),
            id,
            depth,
            fields: Vec::new(),
            definition_lookup: HashMap::new(),
            body,
            scope,
        }
    }
}