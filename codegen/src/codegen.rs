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
use crate::types::{CodegenAccessNode, CodegenBodyStackMode, CodegenExpressionResult, CodegenExpressionStack, CodegenExpressionType, CodegenTrace, CodegenTraceCrumb, CodegenValue, ComptimeType, Field, PrimitiveType, RealtimeValueType, RuntimeVariable, ValueType};

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
                        self.get_type(&func_return_type_node, context_id)?
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
                let field_type_set =  self.get_type(&field_ident, context_id)?;
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

    fn get_type(&mut self, node: &Rc<Node>, context: usize) -> Result<ValueType, CodegenError> {
        let void_register = self.buffer.constant_void();
        let ValueType::Comptime(ComptimeType::Type(construct_field_type_realtime)) = self.generate_expression(context, node, void_register)?.value.value_type else {
            return CodegenError::err(node.clone(), ErrorRepr::ExpectedType);
        };
        return Ok(construct_field_type_realtime.normalize());
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

    fn extract_definition_domain(&self, definition: &CodeDefinition) -> Result<usize, CodegenError> {
        let context = self.extract_definition_context(definition, |f| matches!(f, ContextType::Domain))?;
        
        if !matches!(self.context_borrow(context)?.context_type, ContextType::Domain) {
            return CodegenError::err_headless(ErrorRepr::ExpectedDomain);
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
        let void_register = self.buffer.constant_void();
        // dbg!(void_register);
        let expression = self.generate_expression(context, node, void_register)?;
        let Some(trace) = expression.trace else {
            return CodegenError::err(node.clone(), ErrorRepr::ExpectedAssignableExpression);
        };
        self.set_trace_to_value(trace, CodegenValue::new(get_ident, expression.value.value_type.clone()))?;
        Ok(expression.value.value_type)
    }
    
    fn create_struct_instance_from_node(&mut self, context: usize, construct_ident: &Rc<Node>, construct_body_node: &Rc<Node>, set_ident: u32) -> Result<ValueType, CodegenError> {
        let construct_field_type = self.get_type(construct_ident, context)?;
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
            if field_expression.value.value_type != field_type {
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
            ValueType::Comptime(..) => {
                panic!("The default value of a comptime type shouldn't be scouted for, as a compile-time type (such as Domains, Functions, and Types) doesn't compile to runtime.")
            }
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


    /// Used in generate expression's primary identifier - looks first in the local scope (context), then the parent scope,
    /// then the parent of that, until it looks on main.
    /// 
    /// The priority of which definition types it looks for is this:
    /// - Primitive Types
    /// - Runtime Variables
    /// - Domains
    /// - Functions
    /// - Struct Types
    /// - Fields (for domains that'd be global variables, for structs it'd be fields)
    /// 
    /// in a function it'd first look for the runtime variables in that function,
    /// then go to the parent scope and look for domains first, then fields
    /// then it'd go to that parent scope etc.
    /// 
    /// in a domain it'd first look for the domains inside it, then the fields inside it,
    /// then it'd go to the parent scope and look for its domains first, then its fields, etc.
    /// 
    /// In a struct function it'd first look for the runtime variables, then the struct fields,
    /// then the parent scope (of the struct, not function)'s domains, then its fields, etc.
    fn get_definition_access_isolated(&mut self, context: usize, node: &Rc<Node>, register_group: u64) -> Result<CodegenExpressionResult, CodegenError> {
        let mut current_context = context;
        
        // Primitive Types
        let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedVariableIdentifier)?;
        if let Some(primitive) = Self::is_definition_primitive(var_name.as_str()) {
            return Ok(CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Type(RealtimeValueType::Primitive(primitive)))))
        }

        loop {
            if let Ok(value) = self.get_definition_access(current_context, node, register_group) {
                return Ok(value);
            }
            if current_context == 0 {
                break;
            }
            current_context = self.parents[current_context];
        }
        CodegenError::err(node.clone(), ErrorRepr::DefinitionIdentNotRecognized)
    }

    /// Single version of ``get_definition_access_isolated`` but only on one context, not including its parents.
    fn get_definition_access(&mut self, context: usize, node: &Rc<Node>, register_group: u64) -> Result<CodegenExpressionResult, CodegenError> {
        let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedVariableIdentifier)?;
        // Runtime variables
        if let Some(var) = self.runtime_vars[context].get(var_name) {
            return Ok(CodegenExpressionResult::trace(
                var.variable.clone(),
                CodegenTrace {
                    root_ident: var.variable.ident,
                    crumbs: Vec::new()
                }
            ));
        }
        
        if let Ok(definition) = &self.find_definition_by_ident(node, context) {
            // Domain
            if let Ok(domain_id) = self.extract_definition_domain(definition) {
                return Ok(CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Domain(domain_id))))
            } 
            
            // Function
            if let Ok(func_id) = self.extract_definition_function(definition) {
                return Ok(CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Function(func_id))))
            } 
            
            // Struct (Type)
            if let Ok(struct_id) = self.extract_definition_struct(definition) {
                return Ok(CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Type(RealtimeValueType::Struct(struct_id)))))
            } 
        }
        //TODO: Add fields here
        
        CodegenError::err(node.clone(), ErrorRepr::DefinitionIdentNotRecognized)
    }

    fn set_trace_to_value(&mut self, trace: CodegenTrace, value: CodegenValue) -> Result<(), CodegenError> {
        if trace.crumbs.len() == 0 {
            self.buffer.code_buffer.push_instruction(instruction!(
                Var::Set, [(Ident, trace.root_ident), (Ident, value.ident)]
            ));
            return Ok(());
        }
        let register_group = self.buffer.allocate_line_register_group();
        let trace_ident = trace.root_ident;
        self.set_trace_to_value_inside(Rc::new(trace), trace_ident, value.ident, 0, register_group);
        self.buffer.free_line_register_group(register_group);
        Ok(())
    }

    fn set_trace_to_value_inside(&mut self, trace: Rc<CodegenTrace>, set_value_ident: u32, final_value_ident: u32, depth: usize, register_group: u64) {
        if depth < trace.crumbs.len() {
            let register = self.buffer.allocate_grouped_line_register(register_group);
            self.get_crumb(trace.crumbs[depth].clone(), register, set_value_ident);
            self.set_trace_to_value_inside(trace.clone(), register, final_value_ident, depth+1, register_group);
            self.set_crumb(trace.crumbs[depth].clone(), register, set_value_ident);
        } else { //final one, just set
            self.buffer.code_buffer.push_instruction(instruction!(
                Var::Set, [(Ident, set_value_ident), (Ident, final_value_ident)]
            ));
        }
    }

    fn get_crumb(&mut self, crumb: CodegenTraceCrumb, get_value_ident: u32, set_value_ident: u32) {
        self.buffer.code_buffer.push_instruction(match crumb {
            CodegenTraceCrumb::Index(index) => instruction!(
                Var::GetListValue, [(Ident, get_value_ident), (Ident, set_value_ident), (Int, index)]
            )
        });
    }

    fn set_crumb(&mut self, crumb: CodegenTraceCrumb, get_value_ident: u32, set_value_ident: u32) {
        self.buffer.code_buffer.push_instruction(match crumb {
            CodegenTraceCrumb::Index(index) => instruction!(
                Var::SetListValue, [(Ident, set_value_ident), (Int, index), (Ident, get_value_ident)]
            )
        });
    }

    fn generate_expression(&mut self, context: usize, root_node: &Rc<Node>, set_ident: u32) -> Result<CodegenExpressionResult, CodegenError> {
        //TODO: Refactor by removing ``set_ident`` as the workaround, and moving to register group based stuff
        // means a lot of refactoring needs to be done for stuff that *uses* generate_expression, but i think it's worth it

    
        //##println!("{:#?}", root_node);
        let mut expression_stack= VecDeque::new();
        expression_stack.push_back(CodegenExpressionStack::Node(root_node));
        let register_group = self.buffer.allocate_line_register_group();
        let result = self.generate_expression_inside(context, root_node, register_group)?;
        self.buffer.free_line_register_group(register_group);
        if set_ident != result.value.ident {
            self.buffer.code_buffer.push_instruction(instruction!(
                Var::Set, [(Ident, set_ident), (Ident, result.value.ident)]
            ));
        }
        Ok(result)
    }

    fn generate_expression_inside(&mut self, context: usize, node: &Rc<Node>, register_group: u64) -> Result<CodegenExpressionResult, CodegenError> {
        let mut value = CodegenValue::default();
        let mut trace = None;
        match node.as_ref() {
            Node::Primary(token) => {
                value = match &token.token_type {
                    TokenType::Ident(ident) => {
                        let get_var = self.get_definition_access_isolated(context, node, register_group)?;
                        trace = get_var.trace;
                        get_var.value
                    },
                    TokenType::String(string) => CodegenValue::new(
                        self.buffer.use_string(string.as_str()),
                        ValueType::Primitive(PrimitiveType::String)
                    ),
                    TokenType::Number(number) => CodegenValue::new(
                        self.buffer.use_number(ParameterValue::Float(*number)),
                        ValueType::Primitive(PrimitiveType::Number)
                    ),
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::UnexpectedExpressionToken); }
                }
            }
            Node::Construct(construct_ident, construct_body) => {
                let register = self.buffer.allocate_grouped_line_register(register_group);
                let created_struct = self.create_struct_instance_from_node(context, construct_ident, construct_body, register)?;
                value = CodegenValue::new(register, created_struct);
            }
            Node::FunctionCall(func_ident, func_params) => {
                let register = self.buffer.allocate_grouped_line_register(register_group);
                let func_type = self.call_function(context, func_ident, func_params, register)?;
                value = CodegenValue::new(register, func_type);
            }
            Node::Access(accessed, access_field) => {
                let accessed_expression = self.generate_expression_inside(context, accessed, register_group)?.clone();
                let accessed_value = accessed_expression.value;
                let access_field_ident = Self::get_primary_as_ident(access_field, ErrorRepr::ExpectedAccessableIdentifier)?;

                
                match accessed_value.value_type {
                    ValueType::Struct(struct_id) => {
                        let register = self.buffer.allocate_grouped_line_register(register_group);
                        value.ident = register;
                        let struct_context = self.context_borrow(struct_id)?;
                        let field_id = self.extract_definition_field(
                            struct_context
                            .definition_lookup
                            .get(access_field_ident)
                            .ok_or(CodegenError::new(access_field.clone(), ErrorRepr::InvalidStructField))?
                        )?;
                        let field_type = struct_context.fields[field_id].field_type.clone();
                        drop(struct_context);
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::GetListValue, [ (Ident, register), (Ident, accessed_value.ident), (Int, field_id) ]
                        ));
                        if let Some(mut trace_add) = accessed_expression.trace {
                            trace_add.crumbs.push(CodegenTraceCrumb::Index(field_id));
                            trace = Some(trace_add);
                        }

                        value.value_type = field_type;
                    }
                    ValueType::Comptime(ComptimeType::Domain(domain_cid)) => {
                        let get_access = self.get_definition_access(domain_cid, access_field, register_group)?;
                        value = get_access.value;
                        trace = get_access.trace;
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion); }
                }
            }
            Node::Sum(l, r) => {
                let register = self.buffer.allocate_grouped_line_register(register_group);
                let l = self.generate_expression_inside(context, l, register_group)?.value.clone();
                let r = self.generate_expression_inside(context, r, register_group)?.value.clone();
                value.ident = register;
                match (l.value_type, r.value_type) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Add, [ (Ident, register), (Ident, l.ident), (Ident, r.ident) ]
                        ));
                        value.value_type = ValueType::Primitive(PrimitiveType::Number);
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion); }
                }
            }
            Node::Product(l, r) => {
                let register = self.buffer.allocate_grouped_line_register(register_group);
                let l = self.generate_expression_inside(context, l, register_group)?.value.clone();
                let r = self.generate_expression_inside(context, r, register_group)?.value.clone();
                value.ident = register;
                match (l.value_type, r.value_type) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Mul, [ (Ident, register), (Ident, l.ident), (Ident, r.ident) ]
                        ));
                        value.value_type = ValueType::Primitive(PrimitiveType::Number);
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion); }
                }
            }
            _ => { return CodegenError::err(node.clone(), ErrorRepr::UnexpectedExpressionToken); }
        }
        Ok(CodegenExpressionResult {
            value,
            trace
        })
    }

    fn declare_runtime_variable(&mut self, context: usize, decl_type: &Rc<Node>, decl_ident: &Rc<Node>) -> Result<(&RuntimeVariable, String), CodegenError> {
        let decl_ident_str = Self::get_primary_as_ident(decl_ident, ErrorRepr::ExpectedVariableIdentifier)?;
        let decl_type_str = self.get_type(decl_type, context)?;
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
        // let func_context = self.find_function_by_node(function_ident, context)?;
        let void = self.buffer.constant_void();
        let function_ident_evaluation = self.generate_expression(context, function_ident, void)?;
        let ValueType::Comptime(ComptimeType::Function(func_context)) = function_ident_evaluation.value.value_type else {
            return CodegenError::err(function_ident.clone(), ErrorRepr::ExpectedFunctionIdentifier);
        };
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
            if param_expression.value.value_type != param_field.field_type {
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
                    let register_group = self.buffer.allocate_line_register_group();
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
                            let allocate = self.buffer.allocate_grouped_line_register(register_group);
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

                    if runtime_var_field_type.is_some_and(|field_type| field_type != expr_id.value.value_type) {
                        return CodegenError::err(statement.clone(), ErrorRepr::InvalidVariableType)
                    }
                    self.buffer.free_line_register_group(register_group);
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
                    if !matches!(expr_id.value.value_type, ValueType::Primitive(PrimitiveType::Bool)) {
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
                        if expr_id.value.value_type != return_type {
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
                    if !matches!(expr_id.value.value_type, ValueType::Primitive(PrimitiveType::Bool)) {
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
        self.buffer.clear();
        let _root_context = self.scan_block_outline(node, ContextType::Domain, 0, 0, CodeScope::Public, Vec::new(), "main".to_owned())?;
        self.fill_all_field_types()?;
        self.root_context = 0;
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
        let name = "refactor";
        // let path = r"C:\Users\koren\OneDrive\Documents\Github\Esh\codegen\examples\";
        let path = r"K:\Programming\GitHub\Esh\codegen\examples\";

        let file_bytes = fs::read(format!("{}{}.esh", path, name)).expect("File should read");
        let lexer = Lexer::new(str::from_utf8(&file_bytes).expect("Should encode to utf-8"));
        let lexer_tokens: Vec<Rc<Token>> = lexer.map(|v| Rc::new(v.expect("Lexer token should unwrap"))).collect();
        //##println!("LEXER TOKENS\n----------------------\n{:#?}\n----------------------", lexer_tokens);
        let mut parser = Parser::new(lexer_tokens.as_slice());
        let parser_tree = Rc::new(parser.parse().expect("Parser statement block should unwrap"));
        println!("PARSER TREE\n----------------------\n{:#?}\n----------------------", parser_tree);

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