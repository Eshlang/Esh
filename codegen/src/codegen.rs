use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use dfbin::enums::{Instruction, Parameter, ParameterValue};
use dfbin::instruction;
use dfbin::Constants::Tags::DP;
use lexer::types::{Keyword, TokenType};
use parser::parser::Node;
use crate::buffer::CodeGenBuffer;
use crate::errors::{CodegenError, ErrorRepr};
use crate::context::{CodeDefinition, CodeScope, Context, ContextType};
use crate::types::{CodegenAccessNode, CodegenBodyStackMode, CodegenExpressionStack, CodegenExpressionType, CodegenValue, Field, PrimitiveType, RuntimeVariable, ValueType};

pub struct CodeGen {
    pub context_map: HashMap<String, usize>,
    pub root_context: usize,
    pub contexts: Vec<Rc<RefCell<Context>>>,

    current_id: usize,
    buffer: CodeGenBuffer,
    parents: Vec<usize>,
    runtime_vars: Vec<HashMap<String, RuntimeVariable>>,
    return_runtimes: Vec<Option<RuntimeVariable>>,
    field_names: Vec<Vec<String>>,
    context_names: Vec<String>,
    context_full_names: Vec<String>,
    run: usize,
}

#[allow(dead_code)]
impl CodeGen {
    pub fn new() -> CodeGen {
        Self {
            context_map: HashMap::new(),
            root_context: 0,
            contexts: Vec::new(),
            current_id: 0,
            buffer: CodeGenBuffer::new(),
            parents: Vec::new(),
            runtime_vars: Vec::new(),
            return_runtimes: Vec::new(),
            field_names: Vec::new(),
            context_names: Vec::new(),
            context_full_names: Vec::new(),
            run: 0
        }
    }

    fn scan_block_outline(&mut self, node_block: Rc<Node>, context_type: ContextType, mut depth: u32, parent_id: usize, scope: CodeScope, fields_base: Vec<String>, context_name: String) -> Result<usize, CodegenError> {
        self.run += 1;
        let Node::Block(node_block) = node_block.as_ref() else {
            return CodegenError::err(node_block, ErrorRepr::ExpectedBlock);
        };
        let current_id = self.current_id;
        //##println!("{:?}, {:?}", current_id, depth);
        let current_context_cell = Rc::new(RefCell::new(Context::new_empty(context_type.clone(), parent_id, current_id, depth, Rc::new(Vec::new()), scope)));
        depth += 1;
        self.contexts.push(current_context_cell.clone());
        self.parents.push(parent_id);
        self.runtime_vars.push(HashMap::new());
        self.return_runtimes.push(None);
        self.field_names.push(Vec::new());
        let mut field_names = fields_base;
        self.context_names.push(context_name.clone());
        self.context_full_names.push(if parent_id == current_id {
            let mut str = String::from("__");
            str.push_str(&current_id.to_string());
            str.push('#');
            str.push_str(&context_name);
            str
        } else {
            let str_parent = self.context_full_names.get(parent_id).expect("Parent context full name should exist.");
            let mut str = String::from("__");
            str.push_str(&current_id.to_string());
            str.push('#');
            str.push_str(&str_parent[(str_parent.find('#').expect("String parent should have a #.")+1)..str_parent.len()]);
            str.push('.');
            str.push_str(&context_name);
            str
        });
        let mut current_context = current_context_cell.borrow_mut();
        self.current_id += 1;
        let mut body = Vec::new();
        for node in node_block {
            match (node.as_ref(), &context_type) {
                (Node::Struct(..) | Node::Func(..) | Node::Domain(..), ContextType::Function(..)) => {
                    return CodegenError::err(node.clone(), match node.as_ref() {
                        Node::Struct(..) => ErrorRepr::StructNestedInFunction,
                        Node::Func(..) => ErrorRepr::FunctionNestedInFunction,
                        Node::Domain(..) => ErrorRepr::DomainNestedInFunction,
                        _ => ErrorRepr::Generic
                    });
                }
                (Node::Func(ident, params, return_type, body), ContextType::Struct | ContextType::Domain) => {
                    let return_type_field = ValueType::Ident(return_type.clone());
                    let params = Self::extract_declaration_vec(params)?;
                    let func_fields_base = {
                        let mut res = Vec::new();
                        for (_param_type, param_name) in params.iter() {
                            let param_name_ident = Self::get_primary_as_ident(param_name, ErrorRepr::ExpectedFunctionParamIdent)?;
                            res.push(param_name_ident.clone());
                        }
                        res
                    };
                    let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedFunctionIdentifier)?;
                    let child_id = self.scan_block_outline(body.clone(), ContextType::Function(return_type_field), depth, current_id, CodeScope::Public, func_fields_base, ident_string.clone())?;

                    let mut child_modify = self.context_borrow_mut(child_id)?;
                    for (param_type, param_name) in params {
                        let param_name_ident = Self::get_primary_as_ident(param_name, ErrorRepr::ExpectedFunctionParamIdent)?;
                        let field_id = child_modify.fields.len();
                        child_modify.fields.push(Field {
                            field_type: ValueType::Ident(param_type.clone()),
                            scope: CodeScope::Public,
                        });
                        Self::add_definition(&mut child_modify, param_name_ident.clone(), CodeDefinition::Field(field_id))?;
                    }
                    drop(child_modify);
                    Self::add_definition(&mut current_context, ident_string.clone(), CodeDefinition::Context(child_id))?;
                    current_context.children.push(child_id);
                },
                (Node::Struct(ident, body), _) => {
                    let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedStructIdentifier)?;
                    let child_id = self.scan_block_outline(body.clone(), ContextType::Struct, depth, current_id, CodeScope::Public, Vec::new(), ident_string.clone())?;
                    Self::add_definition(&mut current_context, ident_string.clone(), CodeDefinition::Context(child_id))?;
                    current_context.children.push(child_id);
                },
                (Node::Declaration(field_type, field_name), ContextType::Struct) => {
                    let field_name_ident = Self::get_primary_as_ident(field_name, ErrorRepr::ExpectedStructFieldIdentifier)?;
                    let field_id = current_context.fields.len();
                    field_names.push(field_name_ident.clone());
                    Self::add_definition(&mut current_context, field_name_ident.clone(), CodeDefinition::Field(field_id))?;
                    current_context.fields.push(Field{
                        field_type: ValueType::Ident(field_type.clone()),
                        scope: CodeScope::Public,
                    })
                }
                (Node::Domain(ident, body), ContextType::Domain) => {
                    let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedDomainIdentifier)?;
                    let child_id = self.scan_block_outline(body.clone(), ContextType::Domain, depth, current_id, CodeScope::Public, Vec::new(), ident_string.clone())?;
                    Self::add_definition(&mut current_context, ident_string.clone(), CodeDefinition::Context(child_id))?;
                    current_context.children.push(child_id);
                },
                (Node::Domain(..), ContextType::Struct) => {
                    return CodegenError::err(node.clone(), ErrorRepr::DomainNestedInStruct);
                },
                (_, ContextType::Struct) => {
                    return CodegenError::err(node.clone(), ErrorRepr::UnstructuredStructCode);
                }
                _ => {
                    body.push(node.clone());
                }
            };
        };
        //##println!("WOWWWWW {:?}\n{:?}\n\n", current_id, field_names);
        self.field_names[current_id] = field_names;
        current_context.body = Rc::new(body);
        
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

    fn extract_parameter_vec(node: &Rc<Node>) -> Result<Vec<Rc<Node>>, CodegenError> {
        match node.as_ref() {
            Node::None => Ok(Vec::new()),
            Node::Primary(..) => Ok(vec![node.clone()]),
            Node::Tuple(node_children) => {
                Ok(node_children.clone())
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
                if let ValueType::Ident(func_return_type_node) = func_return_type {
                    let return_type_set = if !matches!(func_return_type_node.as_ref(), Node::None) {
                        self.find_type_by_ident(&func_return_type_node, context_id)?
                    } else { // No return type
                        ValueType::Primitive(PrimitiveType::None)
                    };
                    let mut context_get_mut = self.context_borrow_mut(context_id)?;
                    context_get_mut.context_type = ContextType::Function(return_type_set);
                    drop(context_get_mut);
                };
            }
            for field in 0..fields_len {
                let context_get = self.context_borrow(context_id)?;
                let ValueType::Ident(field_ident) = context_get.fields.get(field).unwrap().field_type.clone() else {
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

    fn find_type_by_ident(&self, ident: &Rc<Node>, context: usize) -> Result<ValueType, CodegenError> {
        let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedTypeIdent)?;
        match Self::is_definition_primitive(ident_string) {
            Some(primitive) => Ok(ValueType::Primitive(primitive)),
            None => {
                let definition = self.find_definition_by_node(ident, context)?;
                let definition = self.extract_definition_struct(&definition)?;
                Ok(ValueType::Struct(definition))
            }
        }
    }

    fn find_function_by_node(&self, node: &Rc<Node>, context: usize) -> Result<usize, CodegenError> {
        let definition = self.find_definition_by_node(node, context)?;
        Ok(self.extract_definition_function(&definition)?)
    }

    fn find_function_by_ident(&self, ident: &Rc<Node>, context: usize) -> Result<usize, CodegenError> {
        let definition = self.find_definition_by_ident(ident, context)?;
        Ok(self.extract_definition_function(&definition)?)
    }

    fn is_definition_primitive(ident_string: &str) -> Option<PrimitiveType> {
        match ident_string {
            "num" => Some(PrimitiveType::Number),
            "string" => Some(PrimitiveType::String),
            "bool" => Some(PrimitiveType::Bool),
            _ => None
        }
    }

    fn extract_definition_context(&self, definition: &CodeDefinition, filter: fn(&ContextType) -> bool) -> Result<usize, CodegenError> {
        match definition {
            CodeDefinition::Context(context) => {
                if filter(&self.context_borrow(*context)?.context_type) {
                    Ok(*context)
                } else {
                    CodegenError::err_headless(ErrorRepr::UnexpectedContextType)
                }
            },
            CodeDefinition::Multiple(definitions) => {
                let mut result = CodegenError::err_headless(ErrorRepr::UnexpectedContextType);
                for check_definition in definitions {
                    if let CodeDefinition::Context(context) = check_definition {
                        if !filter(&self.context_borrow(*context)?.context_type) {
                            continue;
                        }
                        result = Ok(*context);
                        break;
                    }
                }
                result
            },
            _ => CodegenError::err_headless(ErrorRepr::ExpectedContext)
        }
    }

    fn extract_definition_field(&self, definition: &CodeDefinition) -> Result<usize, CodegenError> {
        match definition {
            CodeDefinition::Field(field) => Ok(*field),
            CodeDefinition::Multiple(definitions) => {
                let mut result = CodegenError::err_headless(ErrorRepr::ExpectedField);
                for check_definition in definitions {
                    if let CodeDefinition::Field(field) = check_definition {
                        result = Ok(*field);
                        break;
                    }
                }
                result
            },
            _ => CodegenError::err_headless(ErrorRepr::ExpectedField)
        }
    }

    fn extract_definition_struct(&self, definition: &CodeDefinition) -> Result<usize, CodegenError> {
        let context = self.extract_definition_context(definition, |f| matches!(f, ContextType::Struct))?;
        
        if !matches!(self.context_borrow(context)?.context_type, ContextType::Struct) {
            return CodegenError::err_headless(ErrorRepr::ExpectedStruct);
        }
        Ok(context)
    }

    fn extract_definition_function(&self, definition: &CodeDefinition) -> Result<usize, CodegenError> {
        let context = self.extract_definition_context(definition, |f| matches!(f, ContextType::Function(..)))?;
        if !matches!(self.context_borrow(context)?.context_type, ContextType::Function(..)) {
            return CodegenError::err_headless(ErrorRepr::ExpectedFunction);
        }
        Ok(context)
    }

    fn find_definition_by_node(&self, mut node: &Rc<Node>, mut context: usize) -> Result<CodeDefinition, CodegenError> {
        // access stuff
        let no_depth = if let Node::Access(access_parent, access_child) = node.as_ref() {
            context = self.find_full_context_by_ident(access_parent, context)?;
            node = access_child;
            true
        } else {
            false
        };
        
        
        let ident_string = Self::get_primary_as_ident(node, ErrorRepr::ExpectedDefinitionIdent)?;
        loop {
            let context_borrow = self.context_borrow(context)?;
            match context_borrow.definition_lookup.get(ident_string) {
                Some(def) => {
                    return Ok(def.clone());
                }
                None => {
                    if context_borrow.id == context_borrow.parent_id || no_depth {
                        return CodegenError::err(node.clone(), ErrorRepr::DefinitionIdentNotRecognized)
                    }
                    context = context_borrow.parent_id;
                }
            };
            drop(context_borrow);
        }
    }

    fn find_definition_by_ident(&self, ident: &Rc<Node>, context: usize) -> Result<CodeDefinition, CodegenError> {
        let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedDefinitionIdent)?;
        let context_borrow = self.context_borrow(context)?;
        return Ok(context_borrow.definition_lookup
            .get(ident_string)
            .ok_or(CodegenError::new(ident.clone(), ErrorRepr::DefinitionIdentNotRecognized))?.clone());
    }

    fn find_full_context_by_ident(&self, ident: &Rc<Node>, mut context: usize) -> Result<usize, CodegenError> {
        match ident.as_ref() {
            Node::Primary(_token) => {
                Ok(self.find_domain_by_ident(ident, context, None)?)
            }
            Node::Access(access_parent, access_field) => {
                context = self.find_full_context_by_ident(access_parent, context)?;
                Ok(self.find_domain_by_ident(access_field, context, Some(1))?)
            }
            _ => {
                CodegenError::err(ident.clone(), ErrorRepr::ExpectedAccessableNode)
            }
        }
    }

    fn find_domain_by_ident(&self, ident: &Rc<Node>, mut context: usize, max_depth: Option<usize>) -> Result<usize, CodegenError> {
        let ident_string = Self::get_primary_as_ident(ident, ErrorRepr::ExpectedDefinitionIdent)?;
        let mut depth = 0;
        loop {
            let context_borrow = self.context_borrow(context)?;
            if let Some(CodeDefinition::Context(def_context)) = context_borrow.definition_lookup.get(ident_string) {
                if matches!(self.context_borrow(*def_context)?.context_type, ContextType::Domain) {
                    return Ok(*def_context);
                }
            }
            if context_borrow.id == context_borrow.parent_id {
                return CodegenError::err(ident.clone(), ErrorRepr::DomainIdentNotRecognized);
            }
            context = context_borrow.parent_id;

            depth += 1;
            if let Some(max_depth_value) = max_depth {
                if depth >= max_depth_value {
                    return CodegenError::err(ident.clone(), ErrorRepr::DomainIdentNotRecognized);
                }
            }
            drop(context_borrow);
        }
    }

    fn generate_all_code(&mut self) -> Result<(), CodegenError> {
        for context_id in 0..self.contexts.len() {
            self.generate_code(context_id)?;
        }
        Ok(())
    }

    fn generate_code(&mut self, context: usize) -> Result<(), CodegenError> {
        let (body, context_type, fields) = {
            let context_borrow = self.context_borrow(context)?;
            //##println!("\nGenerating for {:?}\nContext Type: {:?}\n\n", context_borrow.id, context_borrow.context_type);
            (context_borrow.body.clone(), context_borrow.context_type.clone(), context_borrow.fields.clone())
        };
        match context_type {
            ContextType::Struct => {

            },
            ContextType::Function(return_type) => {
                self.generate_function_code(context, body, fields, return_type)?;
            },
            ContextType::Domain => {

            },
        }
        Ok(())
    }
    
    fn make_var_name(var_name: &str, base_char: &str) -> String {
        let mut result = String::from('_');
        result.push_str(base_char);
        result.push('_');
        result.push_str(var_name);
        result
    }

    fn find_variable_by_name_full(&mut self, mut context: usize, var_name: &str, node: &Rc<Node>) -> Result<&RuntimeVariable, CodegenError> {
        loop {
            if let Some(var) = self.runtime_vars[context].get(var_name) {
                return Ok(var);
            }
            if context == 0 {
                return CodegenError::err(node.clone(), ErrorRepr::InvalidVariableName)
            }
            context = self.parents[context];
        }
    }

    fn find_variable_by_name(&mut self, context: usize, node: &Rc<Node>) -> Result<&RuntimeVariable, CodegenError> {
        let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedVariableIdentifier)?;
        self.find_variable_by_name_full(context, var_name, node)
    }

    fn set_variable_to_ident(&mut self, context: usize, node: &Rc<Node>, set_ident: u32) -> Result<ValueType, CodegenError> {
        match node.as_ref() {
            Node::Primary(..) => {
                let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedVariable)?;
                let runtime_var = self.find_variable_by_name_full(context, var_name, node)?.variable.clone();
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, set_ident), (Ident, runtime_var.ident)
                ]));
                Ok(runtime_var.value_type)
            }
            Node::Access(access_parent, access_field) => {
                let accessed_field_type = self.set_variable_to_ident(context, access_parent, set_ident)?;
                let ValueType::Struct(accessed_struct_id) = accessed_field_type else {
                    return CodegenError::err(access_parent.clone(), ErrorRepr::ExpectedAccessableType);
                };
                let struct_context = self.context_borrow(accessed_struct_id)?;
                let access_field_ident = Self::get_primary_as_ident(access_field, ErrorRepr::ExpectedVariable)?;
                let field_id = self.extract_definition_field(
                    struct_context
                    .definition_lookup
                    .get(access_field_ident)
                    .ok_or(CodegenError::new(access_field.clone(), ErrorRepr::InvalidStructField))?
                )?;
                let field_type = struct_context.fields[field_id].field_type.clone();
                drop(struct_context);
                self.buffer.code_buffer.push_instruction(instruction!(Var::GetListValue, [
                    (Ident, set_ident), (Ident, set_ident), (Int, field_id+1)
                ]));
                Ok(field_type)
            }
            _ => {
                CodegenError::err(node.clone(), ErrorRepr::ExpectedVariable)
            }
        }
    }

    fn set_ident_to_variable(&mut self, context: usize, node: &Rc<Node>, get_ident: u32) -> Result<ValueType, CodegenError> {
        match node.as_ref() {
            Node::Primary(..) => {
                let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedVariable)?;
                let runtime_var = self.find_variable_by_name_full(context, var_name, node)?.variable.clone();
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, runtime_var.ident), (Ident, get_ident)
                ]));
                Ok(runtime_var.value_type)
            }
            Node::Access(..) => {
                let mut allocated_registers = vec![0];
                let mut node_stack = Vec::new();
                let mut loop_access = node;
                loop { // Parsing the recursive access into a linear vector, where the root is *last*
                    match loop_access.as_ref() {
                        Node::Access(loop_access_parent, loop_access_field) => {
                            node_stack.push(loop_access_field);
                            loop_access = loop_access_parent;
                        }
                        Node::Primary(..) => {
                            node_stack.push(loop_access);
                            break;
                        }
                        _ => {
                            return CodegenError::err(node.clone(), ErrorRepr::ExpectedVariable);
                        }
                    }
                }
                let mut previous = CodegenValue::default();
                let mut end_instruction_stack = Vec::new();
                for _ in 0..node_stack.len()-1 {
                    allocated_registers.push(self.buffer.allocate_line_register());
                }
                for (access_node_ind, access_node) in node_stack.into_iter().rev().enumerate() {
                    if access_node_ind == 0 { // Root node
                        let var_name = Self::get_primary_as_ident(access_node, ErrorRepr::ExpectedVariable)?;
                        previous = self.find_variable_by_name_full(context, var_name, access_node)?.variable.clone();
                        allocated_registers[0] = previous.ident;
                    } else {
                        let register = allocated_registers[access_node_ind];
                        let register_next = allocated_registers.get(access_node_ind+1).map(|x| *x);
                        previous = match previous.value_type {
                            ValueType::Struct(parent_struct) => {
                                let struct_context = self.context_borrow(parent_struct)?;
                                let access_field_ident = Self::get_primary_as_ident(access_node, ErrorRepr::ExpectedVariable)?;
                                let field_id = self.extract_definition_field(
                                    struct_context
                                    .definition_lookup
                                    .get(access_field_ident)
                                    .ok_or(CodegenError::new(access_node.clone(), ErrorRepr::InvalidStructField))?
                                )?;
                                let field_type = struct_context.fields[field_id].field_type.clone();
                                drop(struct_context);
                                if register_next.is_none() {
                                    end_instruction_stack.push(instruction!(Var::SetListValue, [
                                        (Ident, previous.ident), (Int, field_id+1), (Ident, get_ident)
                                        ]));
                                } else {
                                    self.buffer.code_buffer.push_instruction(instruction!(Var::GetListValue, [
                                        (Ident, register), (Ident, previous.ident), (Int, field_id+1)
                                    ]));
                                    end_instruction_stack.push(instruction!(Var::SetListValue, [
                                        (Ident, previous.ident), (Int, field_id+1), (Ident, register)
                                    ]));
                                }
                                CodegenValue::new(register, field_type)
                            }
                            _ => {
                                return CodegenError::err(access_node.clone(), ErrorRepr::ExpectedAccessableNode);
                            }
                        };
                    }
                }
                for add_end_instruction in end_instruction_stack.into_iter().rev() {
                    self.buffer.code_buffer.push_instruction(add_end_instruction);
                }
                allocated_registers.remove(0);
                self.buffer.free_line_registers(allocated_registers)?;
                Ok(previous.value_type)
            }
            _ => {
                CodegenError::err(node.clone(), ErrorRepr::ExpectedVariable)
            }
        }
    }
    
    fn create_struct_instance_from_node(&mut self, context: usize, construct_ident: &Rc<Node>, construct_body_node: &Rc<Node>, set_ident: u32) -> Result<ValueType, CodegenError> {
        let construct_field_type = self.find_type_by_ident(construct_ident, context)?;
        let ValueType::Struct(struct_type) = construct_field_type else {
            return CodegenError::err(construct_ident.clone(), ErrorRepr::ExpectedStructIdentifier);
        };
        let Node::Block(construct_body) = construct_body_node.as_ref() else {
            return CodegenError::err(construct_body_node.clone(), ErrorRepr::ExpectedBlock);
        };
        let mut param_map = HashMap::new();
        let mut allocated_registers = Vec::new();
        for construct_statement in construct_body {
            let Node::Assignment(assigned_node, assigned_value) = construct_statement.as_ref() else {
                return CodegenError::err(construct_body_node.clone(), ErrorRepr::ExpectedFieldAssignment);
            };
            let assigned_ident = Self::get_primary_as_ident(assigned_node, ErrorRepr::ExpectedFieldAssignment)?;
            let struct_context = self.context_borrow(struct_type)?;
            let Some(def) = struct_context.definition_lookup.get(assigned_ident) else {
                return CodegenError::err(construct_body_node.clone(), ErrorRepr::ExpectedFieldAssignment);
            };
            let field = self.extract_definition_field(def)?;
            let field_type = struct_context.fields.get(field).expect("Struct context should have this field.").field_type.clone();
            drop(struct_context);
            let field_var_ident = self.buffer.allocate_line_register();
            allocated_registers.push(field_var_ident);
            let field_expression = self.generate_expression(context, assigned_value, field_var_ident)?;
            if field_expression.value_type != field_type {
                return CodegenError::err(assigned_value.clone(), ErrorRepr::UnexpectedStructFieldType)
            }
            param_map.insert(field, field_var_ident);
        }
        self.create_struct_instance(construct_body_node, struct_type, set_ident, param_map)?;
        //println!("Struct: {:#?}", construct_body);
        self.buffer.free_line_registers(allocated_registers)?;
        Ok(construct_field_type)
    }

    fn create_struct_instance(&mut self, node: &Rc<Node>, struct_type: usize, set_ident: u32, field_map: HashMap<usize, u32>) -> Result<(), CodegenError> {
        let mut instruction_push = instruction!(Var::CreateList, [
            (Ident, set_ident)
        ]);
        let fields_len = self.context_borrow(struct_type)?.fields.len();
        // let mut allocated_registers = Vec::new();
        for field_id in 0..fields_len {
            if let Some(ident) = field_map.get(&field_id) {
                instruction_push.params.push(Parameter::from_ident(*ident));
            } else {
                return CodegenError::err(node.clone(), ErrorRepr::ConstructFieldsMissing)
            }
            // let field = self.context_borrow(struct_type)?.fields.get(field_id).expect("field_id should be within confines").field_type.clone(); 
            // let ident = self.buffer.allocate_line_register();
            // self.get_default_type_value(&field, ident)?;
            // instruction_push.params.push(Parameter::from_ident(ident));
            // allocated_registers.push(ident);
        }
        self.buffer.code_buffer.push_instruction(instruction_push);
        Ok(())
    }

    fn get_default_type_value(&mut self, field_type: &ValueType, set_ident: u32) -> Result<(), CodegenError> {
        match field_type {
            ValueType::Ident(..) => {
                return CodegenError::err_headless(ErrorRepr::UnexpectedValueTypeIdent)
            },
            ValueType::Struct(..) => {
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, set_ident), (Int, 0)
                ]))
                // self.create_struct_instance(*struct_ind, set_ident)?;
            }
            ValueType::Primitive(PrimitiveType::Bool | PrimitiveType::Number | PrimitiveType::None) => {
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, set_ident), (Int, 0)
                ]))
            },
            ValueType::Primitive(PrimitiveType::String) => {
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, set_ident), (String, "")
                ]))
            },
        }
        Ok(())
    }

    fn find_variable_by_node(&mut self, context: usize, node: &Rc<Node>) -> Result<&RuntimeVariable, CodegenError> {
        let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedVariableIdentifier)?;
        self.find_variable_by_name_full(context, var_name, node)
    }

    fn get_context_name(&self, context: usize) -> &String {
        &self.context_names[context]
    }

    fn get_context_full_name(&self, context: usize) -> &String {
        &self.context_full_names[context]
    }

    fn flatten_access_nodes(&mut self, node: &Rc<Node>) -> VecDeque<CodegenAccessNode> {
        todo!()
    }

    fn get_shallow_domain_access(&self, context: usize, access: &Rc<Node>) -> Result<Option<usize>, CodegenError> {
        if let Node::Primary(token) = access.as_ref() {
            if matches!(token.token_type, TokenType::Ident(..)) {
                if let CodeDefinition::Context(domain_context) = self.find_definition_by_ident(&access, context)?{
                    if matches!(self.context_borrow(domain_context)?.context_type, ContextType::Domain) {
                        return Ok(Some(domain_context));
                    }
                }
            }
        }
        return Ok(None);
    }

    fn get_domain_access(&self, mut context: usize, access: &mut VecDeque<CodegenAccessNode>) -> Result<usize, CodegenError> {
        let mut cut = 0;
        'total: for (node_index, node) in access.into_iter().enumerate() {
            let CodegenAccessNode::Field(node_primary) = node else {
                break;
            };
            if node_index == 0 { // Root node
                'root_search: loop {
                    match self.get_shallow_domain_access(context, node_primary)? {
                        Some(..) => {
                            break 'root_search;
                        }
                        None => {
                            if context == 0 {
                                break 'total;
                            }
                            context = self.parents[context];
                        }
                    }
                }
            }
            let Some(domain_context) = self.get_shallow_domain_access(context, node_primary)? else {
                break;
            };
            cut += 1;
            context = domain_context;
        }
        drop(access.drain(..cut));
        Ok(context)
    }

    fn get_value_access(&mut self, context: usize, node: &Rc<Node>, set_ident: u32) -> Result<CodegenValue, CodegenError> {
        todo!()
    }


    fn generate_expression(&mut self, context: usize, root_node: &Rc<Node>, set_ident: u32) -> Result<CodegenValue, CodegenError> {
        //##println!("{:#?}", root_node);
        let mut expression_stack= VecDeque::new();
        expression_stack.push_back(CodegenExpressionStack::Node(root_node));
        let mut ident_stack: Vec<CodegenValue> = Vec::new();
        let mut register_counter = 0;
        let mut allocated_registers = Vec::new();
        let mut calculated = false;
        while expression_stack.len() > 0 {
            let get_expr = expression_stack.pop_front().expect("This should pop since len > 0");
            //##println!("\n\n#{} (Ident Stack: {:?})\n----------------\n{:?}", ind_counter, ident_stack, get_expr);
            match get_expr {
                CodegenExpressionStack::Node(node) => {
                    let matches = matches!(node.as_ref(), Node::Primary(..));
                    let ident = if register_counter == 0 || matches {
                        set_ident
                    } else {
                        let allocate = self.buffer.allocate_line_register();
                        allocated_registers.push(allocate);
                        allocate
                    };
                    match node.as_ref() {
                        Node::Access(..) => {
                            let field_type = self.set_variable_to_ident(context, node, ident)?;
                            ident_stack.push(CodegenValue::new(ident, field_type.clone()));
                            calculated = true;
                        }
                        Node::Primary(token) => {
                            let variable = match &token.as_ref().token_type {
                                TokenType::Ident(..) => {
                                    self.find_variable_by_name(context, node)?.variable.clone()
                                },
                                TokenType::String(t) => CodegenValue::new(self.buffer.use_string(t.as_str()), ValueType::Primitive(PrimitiveType::String)),
                                TokenType::Number(t) => CodegenValue::new(self.buffer.use_number(ParameterValue::Float(*t)), ValueType::Primitive(PrimitiveType::Number)),
                                TokenType::Keyword(Keyword::True) => CodegenValue::new(self.buffer.use_number(ParameterValue::Int(1)), ValueType::Primitive(PrimitiveType::Bool)),
                                TokenType::Keyword(Keyword::False) => CodegenValue::new(self.buffer.use_number(ParameterValue::Int(0)), ValueType::Primitive(PrimitiveType::Bool)),
                                _ => {
                                    return CodegenError::err(root_node.clone(), ErrorRepr::UnexpectedExpressionToken)
                                }
                            };
                            ident_stack.push(variable);
                        },
                        Node::FunctionCall(func_ident, func_params) => {
                            let func_type = self.call_function(context, func_ident, func_params, ident)?;
                            ident_stack.push(CodegenValue::new(ident, func_type));
                            calculated = true;
                        },
                        Node::Construct(construct_ident, construct_body) => {
                            let created_struct = self.create_struct_instance_from_node(context, construct_ident, construct_body, ident)?;
                            ident_stack.push(CodegenValue::new(ident, created_struct));
                            calculated = true;
                        },
                        Node::Sum(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Add,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::Difference(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Sub,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::Product(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Mul,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::Equal(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Eq,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::NotEqual(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::NotEq,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::Not(n) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Not,
                                ident, //Register ID & Identifier
                                1 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(n));
                        },
                        Node::Or(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Or,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::And(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::And,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::GreaterThan(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Greater,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::GreaterThanOrEqualTo(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::GreaterEq,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::LessThan(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Lesser,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::LessThanOrEqualTo(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::LesserEq,
                                ident, //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        _ => {}
                    }
                    if !matches {
                        register_counter += 1;
                    }
                }
                CodegenExpressionStack::Calculate(expression_type, register_ident, elms) => {
                    let mut parameters = Vec::new();
                    let mut types = Vec::new();
                    for _i in 0..elms {
                        let ident_from_stack = ident_stack.pop().expect("Ident stack should pop when calculating.");
                        parameters.push(Parameter::from_ident(ident_from_stack.ident));
                        types.push(ident_from_stack.value_type)
                    }
                    calculated = true;
                    let result_type = self.calculate_expression(expression_type, register_ident, parameters, types, root_node)?;
                    ident_stack.push(CodegenValue::new(register_ident, result_type));
                }
                
            }
        }
        let final_value = ident_stack.pop().expect("Ident stack should have a final value.");
        if !calculated { //The expression was a single primary value and included no calculation, so we need to set the intended value.
            self.buffer.code_buffer.push_instruction(instruction!(
                Var::Set, [
                    (Ident, set_ident),
                    (Ident, final_value.ident)
                ]
            ));
        }
        self.buffer.free_line_registers(allocated_registers)?;
        Ok(CodegenValue::new(set_ident, final_value.value_type))
    }

    fn calculate_expression(&mut self, expression_type: CodegenExpressionType, ident: u32, parameters: Vec<Parameter>, types: Vec<ValueType>, root_node: &Rc<Node>) -> Result<ValueType, CodegenError> {
        let result_type = match expression_type {
            CodegenExpressionType::Add => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(PrimitiveType::String), ValueType::Primitive(PrimitiveType::String)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::String, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::String)
                    },
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Add, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::Number)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Sub => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Sub, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::Number)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Mul => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Mul, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::Number)
                    },
                    (ValueType::Primitive(PrimitiveType::String), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::RepeatString, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::String)
                    },
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::String)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::RepeatString, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        ValueType::Primitive(PrimitiveType::String)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Div => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Div, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::Number)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Eq | CodegenExpressionType::NotEq => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(_), ValueType::Primitive(_)) => {
                        self.buffer.code_buffer.push_instruction(
                            match expression_type {
                                CodegenExpressionType::NotEq => instruction!(Varif::NotEq),
                                _ => instruction!(Varif::Eq)
                            }
                        );
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, ident),
                    (Int, 1)
                ]));
                self.buffer.code_buffer.push_instruction(instruction!(Else));
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, ident),
                    (Int, 0)
                ]));
                self.buffer.code_buffer.push_instruction(instruction!(EndIf));
                ValueType::Primitive(PrimitiveType::Bool)
            },
            CodegenExpressionType::Not => { //we're basically doing the operation ``newbool = 1 - oldbool``.
                match types.get(0).unwrap() {
                    ValueType::Primitive(PrimitiveType::Bool) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Sub, [
                                (Ident, ident),
                                (Int, 1)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        ValueType::Primitive(PrimitiveType::Bool)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Or => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(PrimitiveType::Bool), ValueType::Primitive(PrimitiveType::Bool)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Bitwise, [
                                (Ident, ident)
                            ], {
                                Operator: OR
                            }
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::Bool)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::And => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(PrimitiveType::Bool), ValueType::Primitive(PrimitiveType::Bool)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Bitwise, [
                                (Ident, ident)
                            ], {
                                Operator: AND
                            }
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        ValueType::Primitive(PrimitiveType::Bool)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Greater | CodegenExpressionType::GreaterEq | CodegenExpressionType::Lesser | CodegenExpressionType::LesserEq => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(
                            match expression_type {
                                CodegenExpressionType::Greater => instruction!(Varif::Greater),
                                CodegenExpressionType::GreaterEq => instruction!(Varif::GreaterEq),
                                CodegenExpressionType::Lesser => instruction!(Varif::Lower),
                                CodegenExpressionType::LesserEq => instruction!(Varif::LowerEq),
                                _ => instruction!(Varif::Eq)
                            }
                        );
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, ident),
                    (Int, 1)
                ]));
                self.buffer.code_buffer.push_instruction(instruction!(Else));
                self.buffer.code_buffer.push_instruction(instruction!(Var::Set, [
                    (Ident, ident),
                    (Int, 0)
                ]));
                self.buffer.code_buffer.push_instruction(instruction!(EndIf));
                ValueType::Primitive(PrimitiveType::Bool)
            },
        };
        Ok(result_type)
    }

    fn declare_runtime_variable(&mut self, context: usize, decl_type: &Rc<Node>, decl_ident: &Rc<Node>) -> Result<(&RuntimeVariable, String), CodegenError> {
        let decl_ident_str = Self::get_primary_as_ident(decl_ident, ErrorRepr::ExpectedVariableIdentifier)?;
        let decl_type_str = self.find_type_by_ident(decl_type, context)?;
        let var_name = Self::make_var_name(decl_ident_str, "rvl");
        let var_ident = self.buffer.use_variable(var_name.as_ref(), DP::Var::Scope::Line);
        let runtime_str = decl_ident_str.to_owned();
        if self.find_variable_by_name_full(context, &decl_ident_str, decl_ident).is_ok() {
            return CodegenError::err(decl_ident.clone(), ErrorRepr::DeclaringExistingVariable);
        }
        self.runtime_vars[context].insert(
            runtime_str.to_owned(), 
            RuntimeVariable::new(
                CodegenValue::new(var_ident, decl_type_str),
                var_name
            )
        );
        Ok((self.runtime_vars[context].get(&runtime_str).unwrap(), runtime_str))
    }

    fn call_function(&mut self, context: usize, function_ident: &Rc<Node>, function_params: &Rc<Node>, return_ident: u32) -> Result<ValueType, CodegenError> {
        let func_context = self.find_function_by_node(function_ident, context)?;
        let func_name = self.get_context_full_name(func_context).clone();
        let func_id = self.buffer.use_function(func_name.as_str());
        let mut call_instruction = instruction!(Call, [
            (Ident, func_id)
        ]);
        let ContextType::Function(ret_type_field) = self.context_borrow(func_context)?.context_type.clone() else {
            return CodegenError::err_headless(ErrorRepr::Generic);
        };
        if !matches!(ret_type_field, ValueType::Primitive(PrimitiveType::None)) { // function has a return value
            call_instruction.params.push(Parameter::from_ident(return_ident))
        }
        let params = Self::extract_parameter_vec(function_params)?;
        let mut param_var_idents = Vec::new();
        let func_fields = self.context_borrow(func_context)?.fields.clone();
        if params.len() > func_fields.len() {
            return CodegenError::err(function_params.clone(), ErrorRepr::UnexpectedFunctionParameter)
        }
        if params.len() < func_fields.len() {
            return CodegenError::err(function_params.clone(), ErrorRepr::ExpectedFunctionParameter)
        }
        for (param, param_field) in params.into_iter().zip(func_fields) {
            let param_var_ident = self.buffer.allocate_line_register();
            param_var_idents.push(param_var_ident);
            let param_expression = self.generate_expression(context, &param, param_var_ident)?;
            if param_expression.value_type != param_field.field_type {
                return CodegenError::err(param.clone(), ErrorRepr::UnexpectedFunctionParameterType)
            }
            call_instruction.params.push(Parameter::from_ident(param_var_ident));
        }

        self.buffer.code_buffer.push_instruction(call_instruction);
        self.buffer.free_line_registers(param_var_idents)?;
        Ok(ret_type_field)
    }


    fn generate_function_code(&mut self, context: usize, body: Rc<Vec<Rc<Node>>>, fields: Vec<Field>, return_type: ValueType) -> Result<(), CodegenError> {
        // self.return_runtimes[context] = 
        let parent = self.parents[context];
        let parent_context = self.context_borrow(parent)?.context_type.clone();
        
        let func_name = self.get_context_full_name(context).clone();
        let func_id = self.buffer.use_function(func_name.as_str());
        self.buffer.code_buffer.push_instruction(instruction!(
            Func, [
                (Ident, func_id)
            ]
        ));
        //println!("Generating function {}, return type: {:#?}", context, return_type);
        let _self_struct_ident = if matches!(parent_context, ContextType::Struct) {
            let self_struct_param_ident = self.buffer.use_return_param("self_struct");
            self.buffer.code_buffer.push_parameter(Parameter::from_ident(self_struct_param_ident.0));
            Some(self_struct_param_ident.1)
        } else {
            None
        };
        let return_type_ident = if !matches!(return_type, ValueType::Primitive(PrimitiveType::None)) {
            let return_type_param_ident = self.buffer.use_return_param("fr");
            self.buffer.code_buffer.push_parameter(Parameter::from_ident(return_type_param_ident.0));
            Some(return_type_param_ident.1)
        } else {
            None
        };
        let mut returned_value = false;
        let mut field_id = 0;
        for field in fields {
            let var_name = Self::make_var_name(&self.field_names[context][field_id], "rvp");
            let param_and_var_ident = self.buffer.use_param(var_name.as_ref());
            self.runtime_vars[context].insert(
                self.field_names[context][field_id].to_owned(), 
                RuntimeVariable::new_param(
                    field.field_type,
                    var_name,
                    param_and_var_ident
                )
            );
            self.buffer.code_buffer.push_parameter(Parameter::from_ident(param_and_var_ident.0));
            field_id += 1;
        }
        let mut body_stack: VecDeque<(usize, Rc<Vec<Rc<Node>>>, Vec<String>, Option<Instruction>, CodegenBodyStackMode)> = VecDeque::new();
        body_stack.push_back((0, body, Vec::new(), None, CodegenBodyStackMode::None));
        
        'total: loop {
            'verify: loop {
                if body_stack[0].0 >= body_stack[0].1.len() {
                    let remove = body_stack.pop_front().expect("Body stack should pop a front");
                    if let Some(instruction_trail) = remove.3 {
                        self.buffer.code_buffer.push_instruction(instruction_trail);
                    }
                    for remove_variable in remove.2 {
                        self.runtime_vars[context].remove(&remove_variable);
                    }
                    if body_stack.len() == 0 {
                        break 'total;   
                    }
                } else {
                    break 'verify;
                }
            }
            if body_stack.len() == 0 {
                break 'total;   
            }
            body_stack[0].0 += 1;
            let body_get = &body_stack[0];
            let body_stack_mode = body_get.4.clone();
            let statement = (&body_get.1[body_get.0 - 1]).clone();
            let mut block_runtime_vars_add = Vec::new();
            match statement.as_ref() {
                Node::Declaration(decl_type, decl_ident) => {
                    block_runtime_vars_add.push(self.declare_runtime_variable(context, decl_type, decl_ident)?.1);
                },
                Node::Assignment(assign_var, assign_value) => {
                    let mut allocated_registers = Vec::new();
                    let (runtime_var_ident, mut runtime_var_field_type, already_set) = match assign_var.as_ref() {
                        Node::Declaration(decl_type, decl_ident) => {
                            let declaration = self.declare_runtime_variable(context, decl_type, decl_ident)?;
                            block_runtime_vars_add.push(declaration.1);
                            (declaration.0.variable.ident, Some(declaration.0.variable.value_type.clone()), true)
                        }
                        Node::Primary(..) => {
                            let runtime_var = self.find_variable_by_name(context, assign_var)?;
                            (runtime_var.variable.ident, Some(runtime_var.variable.value_type.clone()), true)
                        }
                        Node::Access(..) => {
                            let allocate = self.buffer.allocate_line_register();
                            allocated_registers.push(allocate);
                            (allocate, None, false)
                        }
                        _ => {
                            return CodegenError::err(statement.clone(), ErrorRepr::InvalidAssignmentToken);
                        }
                    };
                    let expr_id = self.generate_expression(context, assign_value, runtime_var_ident)?;
                    if !already_set {
                        runtime_var_field_type = Some(self.set_ident_to_variable(context, assign_var, runtime_var_ident)?);
                    }
                    if runtime_var_field_type.is_some_and(|field_type| field_type != expr_id.value_type) {
                        return CodegenError::err(statement.clone(), ErrorRepr::InvalidVariableType)
                    }
                    self.buffer.free_line_registers(allocated_registers)?;
                },
                Node::Else(if_node, else_block) => {
                    let Node::Block(else_block) = else_block.as_ref() else {
                        return CodegenError::err(else_block.clone(), ErrorRepr::ExpectedBlock);
                    };
                    body_stack.push_front((0, Rc::new(else_block.clone()), Vec::new(), Some(instruction!(EndIf)), body_stack_mode));
                    // Do the if stuff
                    body_stack.push_front((0, Rc::new(vec![if_node.clone()]), Vec::new(), None, CodegenBodyStackMode::Else));
                },
                Node::If(if_condition, if_block) => {
                    let allocated_bool = self.buffer.allocate_line_register();
                    let expr_id = self.generate_expression(context, if_condition, allocated_bool)?;
                    if !matches!(expr_id.value_type, ValueType::Primitive(PrimitiveType::Bool)) {
                        return CodegenError::err(statement.clone(), ErrorRepr::InvalidVariableType)
                    }
                    self.buffer.code_buffer.push_instruction(instruction!(
                        Varif::Eq, [
                            (Ident, allocated_bool),
                            (Int, 1)
                        ]
                    ));
                    let Node::Block(if_block) = if_block.as_ref() else {
                        return CodegenError::err(if_block.clone(), ErrorRepr::ExpectedBlock);
                    };
                    body_stack.push_front((0, Rc::new(if_block.clone()), Vec::new(), Some(match body_stack_mode {
                        CodegenBodyStackMode::None => instruction!(EndIf),
                        CodegenBodyStackMode::Else => instruction!(Else),
                    }), body_stack_mode));
                    self.buffer.free_line_register(allocated_bool)?;
                },
                Node::Return(return_value) => {
                    if !matches!(return_value.as_ref(), Node::None) { //You're returning a value
                        let Some(return_type_ident_some) = return_type_ident else {
                            return CodegenError::err(return_value.clone(), ErrorRepr::UnexpectedReturnValue)
                        };
                        let expr_id = self.generate_expression(context, return_value, return_type_ident_some)?;
                        if expr_id.value_type != return_type {
                            return CodegenError::err(statement.clone(), ErrorRepr::InvalidReturnValueType)
                        }
                    }
                    self.buffer.code_buffer.push_instruction(instruction!(Ctrl::Return));
                    if body_stack.len() == 1 { // This is the core branch.
                        returned_value = true;
                    }
                },
                Node::FunctionCall(function_ident, function_parameters) => {
                    let void = self.buffer.constant_void();
                    self.call_function(context, function_ident, function_parameters, void)?;
                },
                Node::While(while_cond, while_block) => {
                    self.buffer.code_buffer.push_instruction(instruction!(Rep::Forever));
                    let allocated_bool = self.buffer.allocate_line_register();
                    let expr_id = self.generate_expression(context, while_cond, allocated_bool)?;
                    if !matches!(expr_id.value_type, ValueType::Primitive(PrimitiveType::Bool)) {
                        return CodegenError::err(statement.clone(), ErrorRepr::InvalidVariableType)
                    }
                    self.buffer.code_buffer.push_instruction(instruction!(
                        Varif::Eq, [
                            (Ident, allocated_bool),
                            (Int, 1)
                        ]
                    ));
                    self.buffer.code_buffer.push_instruction(instruction!(Ctrl::StopRepeat));
                    self.buffer.code_buffer.push_instruction(instruction!(EndIf));
                    let Node::Block(if_block) = while_block.as_ref() else {
                        return CodegenError::err(while_block.clone(), ErrorRepr::ExpectedBlock);
                    };
                    body_stack.push_front((0, Rc::new(if_block.clone()), Vec::new(), Some(instruction!(EndRep)), body_stack_mode));
                    self.buffer.free_line_register(allocated_bool)?;
                },
                _ => {
                    println!("Unrecognized Statement: {:#?}", statement.clone());
                }
            }
            body_stack[0].2.extend(block_runtime_vars_add);
        }
        if !returned_value && return_type_ident.is_some() { // This means the function needs to a return a value, but hasn't in the core branch.
            return CodegenError::err_headless(ErrorRepr::ExpectedFunctionReturnValue);
        }
        Ok(())
    }

    pub fn codegen_from_node(&mut self, node: Rc<Node>) -> Result<(), CodegenError> {
        let _root_context = self.scan_block_outline(node, ContextType::Domain, 0, 0, CodeScope::Public, Vec::new(), "main".to_owned())?;
        self.fill_all_field_types()?;
        self.root_context = 0;
        self.buffer.clear();
        //##println!("\n\n\n\n{:#?}\n\n\n\n", self.context_names);
        self.generate_all_code()?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use core::str;
    use std::fs;

    use parser::parser::*;
    use lexer::{Lexer, types::Token};
    use super::*;


    #[test]
    pub fn decompile_from_file_test() {
        let name = "more";
        let path = r"C:\Users\koren\OneDrive\Documents\Github\Esh\codegen\examples\";
        // let path = r"K:\Programming\GitHub\Esh\codegen\examples\";

        let file_bytes = fs::read(format!("{}{}.esh", path, name)).expect("File should read");
        let lexer = Lexer::new(str::from_utf8(&file_bytes).expect("Should encode to utf-8"));
        let lexer_tokens: Vec<Rc<Token>> = lexer.map(|v| Rc::new(v.expect("Lexer token should unwrap"))).collect();
        //##println!("LEXER TOKENS\n----------------------\n{:#?}\n----------------------", lexer_tokens);
        let mut parser = Parser::new(lexer_tokens.as_slice());
        let parser_tree = Rc::new(parser.parse().expect("Parser statement block should unwrap"));
        //##println!("PARSER TREE\n----------------------\n{:#?}\n----------------------", parser_tree);

        let mut codegen = CodeGen::new();
        codegen.codegen_from_node(parser_tree.clone()).expect("Codegen should generate");
        //##println!("CODEGEN CONTEXTS\n----------------------\n{:#?}\n----------------------", codegen.contexts);

        //##println!("CODEGEN CONTEXT NAMES\n----------------------");
        //##for context in 0..codegen.contexts.len() {
        //##    let mut name = codegen.get_context_full_name(context).clone();
        //##    let name = name.split_off(name.find("#").expect("Should have a # in it.") + 1);
        //##    println!("ID: {}, Name: {}", context, name);
        //##}
        //##println!("----------------------");

        let code = codegen.buffer.flush();
        code.write_to_file(&format!("{}{}.dfbin", path, name)).expect("DFBin should write");
        let mut decompiler = decompiler::Decompiler::new(code).expect("Decompiler should create");
        decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
        let decompiled = decompiler.decompile().expect("Decompiler should decompile");
        //##println!("DECOMPILED\n----------------------\n{}\n----------------------", decompiled);
        fs::write(format!("{}{}.dfa", path, name), decompiled).expect("Decompiled DFA should write.");
    }
}