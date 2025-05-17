use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use dfbin::enums::{Instruction, Parameter, ParameterValue};
use dfbin::{instruction, tag};
use dfbin::Constants::Tags::DP;
use lexer::compiler::Compiler;
use lexer::types::{Keyword, Range, Token, TokenType, ValuedKeyword};
use esh_parser::parser::Node;
use crate::buffer::CodeGenBuffer;
use crate::errors::{CodegenError, ErrorRepr};
use crate::context::{CodeDefinition, CodeScope, Context, ContextType};
use crate::types::{CodegenBodyStackMode, CodegenExpressionResult, CodegenExpressionStack, CodegenExpressionType, CodegenLocationCoordinate, CodegenTrace, CodegenTraceCrumb, CodegenTraceCrumbIdent, CodegenValue, CodegenVectorCoordinate, ComptimeType, Field, FieldDefinition, GenerateExpressionSettings, IdentifierCategory, PrimitiveType, RealtimeValueType, RuntimeVariable, ValueType};

pub struct CodeGen {
    pub context_map: HashMap<String, usize>,
    pub root_context: usize,
    pub contexts: Vec<Rc<RefCell<Context>>>,

    current_id: usize,
    buffer: CodeGenBuffer,
    parents: Vec<usize>,
    runtime_vars: Vec<HashMap<String, RuntimeVariable>>,
    domain_vars: Vec<Vec<RuntimeVariable>>,
    return_runtimes: Vec<Option<RuntimeVariable>>,
    field_names: Vec<Vec<String>>,
    context_names: Vec<String>,
    context_full_names: Vec<String>,
    block_runtime_vars_add: Vec<String>,
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
            domain_vars: Vec::new(),
            return_runtimes: Vec::new(),
            field_names: Vec::new(),
            context_names: Vec::new(),
            context_full_names: Vec::new(),
            block_runtime_vars_add: Vec::new(),
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
        let mut domain_vars = Vec::new();
        self.domain_vars.push(Vec::new());
        self.return_runtimes.push(None);
        self.field_names.push(Vec::new());
        let mut field_names = fields_base;
        let mut field_names_hash = HashSet::new();
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
                (Node::Declaration(field_type, field_name), ContextType::Struct | ContextType::Domain) => {
                    let field_name_ident = Self::get_primary_as_ident(field_name, ErrorRepr::ExpectedStructFieldIdentifier)?;
                    let field_id = current_context.fields.len();
                    if field_names_hash.contains(field_name_ident) {
                        return CodegenError::err(field_name.clone(), ErrorRepr::FieldAlreadyDefined);
                    }
                    field_names.push(field_name_ident.clone());
                    field_names_hash.insert(field_name_ident.clone());
                    Self::add_definition(&mut current_context, field_name_ident.clone(), CodeDefinition::Field(field_id))?;
                    current_context.fields.push(Field{
                        field_type: ValueType::Ident(field_type.clone()),
                        scope: CodeScope::Public,
                    });

                    if matches!(&context_type, ContextType::Domain) { //Fields in domains are domain variables
                        let mut format = self.get_context_full_name(current_id).clone();
                        format.remove(0);
                        format.remove(0);
                        format.push('.');
                        format.push_str(&field_name_ident);
                        let var_name = Self::make_var_name(&format, "dvg");
                        let var_ident = self.buffer.use_variable(var_name.as_ref(), DP::Var::Scope::Global);
                        domain_vars.push(
                            RuntimeVariable::new(
                                CodegenValue::new(var_ident, ValueType::Ident(field_type.clone())),
                                var_name
                            )
                        );
                    }
                },
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
        self.domain_vars[current_id] = domain_vars;
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

            let domain_vars_len = self.domain_vars[context_id].len();
            
            for field in 0..fields_len {
                let context_get = self.context_borrow(context_id)?;
                let ValueType::Ident(field_ident) = context_get.fields.get(field).expect("Field should be defined").field_type.clone() else {
                    continue;
                };
                drop(context_get);
                let field_type_set =  self.get_type(&field_ident, context_id)?;
                if field < domain_vars_len { // Also replace the domain_var ident
                    self.domain_vars[context_id][field].variable.value_type = field_type_set.clone();
                }
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
        let ValueType::Comptime(ComptimeType::Type(construct_field_type_realtime)) = self.generate_expression(context, node, GenerateExpressionSettings::comptime().prefer_category(IdentifierCategory::Type))?.value.value_type else {
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
            "vec" => Some(PrimitiveType::Vector),
            "loc" => Some(PrimitiveType::Location),
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

    
    fn create_struct_instance_from_node(&mut self, context: usize, construct_ident: &Rc<Node>, construct_body_node: &Rc<Node>, set_ident: u32) -> Result<ValueType, CodegenError> {
        let construct_field_type = self.get_type(construct_ident, context)?;
        let ValueType::Struct(struct_type) = construct_field_type else {
            return CodegenError::err(construct_ident.clone(), ErrorRepr::ExpectedStructIdentifier);
        };
        let Node::Block(construct_body) = construct_body_node.as_ref() else {
            return CodegenError::err(construct_body_node.clone(), ErrorRepr::ExpectedBlock);
        };
        let mut param_map = HashMap::new();
        let register_group = self.buffer.allocate_line_register_group();
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
            let field_expression = self.generate_expression(context, assigned_value, GenerateExpressionSettings::parameter(register_group).expect_type(&field_type))?;
            if field_expression.value.value_type != field_type {
                return CodegenError::err(assigned_value.clone(), ErrorRepr::UnexpectedStructFieldType)
            }
            param_map.insert(field, field_expression.value.ident);
        }
        self.create_struct_instance(construct_body_node, struct_type, set_ident, param_map)?;
        //println!("Struct: {:#?}", construct_body);
        self.buffer.free_line_register_group(register_group);
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
            ValueType::Primitive(PrimitiveType::List(..)) => {
                self.buffer.code_buffer.push_instruction(instruction!(Var::CreateList, [
                    (Ident, set_ident)
                ]))
            },
            ValueType::Primitive(PrimitiveType::Map(..)) => {
                self.buffer.code_buffer.push_instruction(instruction!(Var::CreateDict, [
                    (Ident, set_ident)
                ]))
            },
            ValueType::Primitive(PrimitiveType::Vector) => {
                self.buffer.code_buffer.push_instruction(instruction!(Var::Vector, [
                    (Ident, set_ident), (Int, 0), (Int, 0), (Int, 0)
                ]))
            },
            ValueType::Primitive(PrimitiveType::Location) => {
                self.buffer.code_buffer.push_instruction(instruction!(Var::SetAllCoords, [
                    (Ident, set_ident), (Int, 0), (Int, 0), (Int, 0), (Int, 0), (Int, 0)
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
    fn get_definition_access_isolated(&mut self, context: usize, settings: &GenerateExpressionSettings, node: &Rc<Node>, register_group: u64) -> Result<CodegenExpressionResult, CodegenError> {
        let mut current_context = context;
        
        // Primitive Types
        let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedVariableIdentifier)?;
        if let Some(primitive) = Self::is_definition_primitive(var_name.as_str()) {
            return Ok(CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Type(RealtimeValueType::Primitive(primitive)))))
        }

        loop {
            if let Ok(value) = self.get_definition_access(current_context, settings, node, register_group) {
                return Ok(value);
            }
            if current_context == 0 {
                break;
            }
            current_context = self.parents[current_context];
        }
        CodegenError::err(node.clone(), ErrorRepr::DefinitionIdentNotRecognized)
    }

    fn get_context_type(&mut self, context: usize) -> Result<ContextType, CodegenError> {
        Ok(self.context_borrow(context)?.context_type.clone())
    }

    /// Single version of ``get_definition_access_isolated`` but only on one context, not including its parents.
    fn get_definition_access(&mut self, context: usize, settings: &GenerateExpressionSettings, node: &Rc<Node>, register_group: u64) -> Result<CodegenExpressionResult, CodegenError> {
        let var_name = Self::get_primary_as_ident(node, ErrorRepr::ExpectedIdentifier)?;
        let mut found = Vec::new();
        
        // Runtime variables
        if let Some(var) = self.runtime_vars[context].get(var_name) {
            found.push((CodegenExpressionResult::trace(
                var.variable.clone(),
                CodegenTrace::root(var.variable.ident)
            ), IdentifierCategory::RuntimeVariable));
        }
        
        if matches!(self.get_context_type(context)?, ContextType::Domain) {
            if let Ok(definition) = &self.find_definition_by_ident(node, context) {
                // Domain
                if let Ok(domain_id) = self.extract_definition_domain(definition) {
                    found.push((CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Domain(domain_id))), IdentifierCategory::Domain));
                } 
                
                // Function
                if let Ok(func_id) = self.extract_definition_function(definition) {
                    found.push((CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Function(func_id))), IdentifierCategory::Function));
                } 
                
                // Struct (Type)
                if let Ok(struct_id) = self.extract_definition_struct(definition) {
                    found.push((CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Type(RealtimeValueType::Struct(struct_id)))), IdentifierCategory::Type));
                } 
                // Fields (Global Variables a.k.a Domain Variables)
                if let Ok(domain_definition) = self.find_context_field(context, node) {
                    let domain_var_value = &self.domain_vars[context][domain_definition.index].variable;
                    found.push((CodegenExpressionResult::trace(
                        domain_var_value.clone(),
                        CodegenTrace::root(domain_var_value.ident)
                    ), IdentifierCategory::Field));
                }
            }
        } 

        for definition in found.iter() {
            if definition.1 == settings.preferred_category {
                return Ok(definition.0.clone());
            }
        }
        Ok(found
            .get(0)
            .ok_or(CodegenError::new(node.clone(), ErrorRepr::DefinitionIdentNotRecognized))?
            .0.clone())
    }

    fn find_context_field(&mut self, context: usize, identifier: &Rc<Node>) -> Result<FieldDefinition, CodegenError> {
        let identifier_string = Self::get_primary_as_ident(identifier, ErrorRepr::ExpectedIdentifier)?;
        let struct_context = self.context_borrow(context)?;
        let field_id = self.extract_definition_field(
            struct_context
            .definition_lookup
            .get(identifier_string)
            .ok_or(CodegenError::new(identifier.clone(), ErrorRepr::InvalidStructField))?
        )?;
        return Ok(FieldDefinition { field: struct_context.fields[field_id].clone(), index: field_id });
    }

    fn set_trace_to_value(&mut self, context: usize, mut trace: CodegenTrace, value: CodegenValue) -> Result<(), CodegenError> {
        if trace.crumbs.len() == 0 {
            self.buffer.code_buffer.push_instruction(instruction!(
                Var::Set, [(Ident, trace.root_ident), (Ident, value.ident)]
            ));
            return Ok(());
        }
        let register_group = self.buffer.allocate_line_register_group();
        let trace_ident = trace.root_ident;
        let mut crumbs = Vec::new();
        for crumb_index in 0..trace.crumbs.len() {
            let crumb = trace.crumbs[crumb_index].clone();
            crumbs.push(self.generate_crumb(context, crumb, register_group)?);
        }
        self.set_trace_to_value_inside(Rc::new(crumbs), trace_ident, value.ident, 0, register_group);
        self.buffer.free_line_register_group(register_group);
        Ok(())
    }

    fn generate_crumb(&mut self, context: usize, crumb: CodegenTraceCrumb, register_group: u64) -> Result<CodegenTraceCrumbIdent, CodegenError> {
        Ok(match crumb {
            CodegenTraceCrumb::Ident(ident) => ident,
            CodegenTraceCrumb::IndexNode(node) => {
                let index = self.generate_expression_inside(context, &node, GenerateExpressionSettings::parameter(register_group), register_group)?.value.clone();
                let register_index = self.buffer.allocate_grouped_line_register(register_group);
                self.buffer.code_buffer.push_instruction(instruction!(
                    Var::Add, [ (Ident, register_index), (Ident, index.ident), (Int, 1) ]
                ));
                CodegenTraceCrumbIdent::Index(register_index)
            },
            CodegenTraceCrumb::EntryNode(node) => {
                let index = self.generate_expression_inside(context, &node, GenerateExpressionSettings::parameter(register_group), register_group)?.value.clone();
                let register_index = if !matches!(index.value_type, ValueType::Primitive(PrimitiveType::String)) {
                    let temp_reg = self.buffer.allocate_grouped_line_register(register_group);
                    self.buffer.code_buffer.push_instruction(instruction!(
                        Var::String, [ (Ident, index.ident), (String, " "), (Ident, temp_reg) ]
                    ));
                    temp_reg
                } else {
                    index.ident
                };
                CodegenTraceCrumbIdent::Entry(register_index)
            }
        })
    }

    fn set_trace_to_value_inside(&mut self, trace: Rc<Vec<CodegenTraceCrumbIdent>>, set_value_ident: u32, final_value_ident: u32, depth: usize, register_group: u64) {
        if depth < trace.len() - 1 {
            let register = self.buffer.allocate_grouped_line_register(register_group);
            self.get_crumb(trace[depth].clone(), register, set_value_ident);
            self.set_trace_to_value_inside(trace.clone(), register, final_value_ident, depth+1, register_group);
            self.set_crumb(trace[depth].clone(), register, set_value_ident);
        } else { //final one, just set
            self.set_crumb(trace[depth].clone(), final_value_ident, set_value_ident);
        }
    }

    fn get_crumb(&mut self, crumb: CodegenTraceCrumbIdent, value_ident: u32, accesed_ident: u32) {
        self.buffer.code_buffer.push_instruction(match crumb {
            CodegenTraceCrumbIdent::IndexDirect(index) => instruction!(
                Var::GetListValue, [(Ident, value_ident), (Ident, accesed_ident), (Int, index)]
            ),
            CodegenTraceCrumbIdent::Index(ident) => instruction!(
                Var::GetListValue, [(Ident, value_ident), (Ident, accesed_ident), (Ident, ident)]
            ),
            CodegenTraceCrumbIdent::Entry(ident) => instruction!(
                Var::GetDictValue, [(Ident, value_ident), (Ident, accesed_ident), (Ident, ident)]
            ),
            CodegenTraceCrumbIdent::Location(coord) => instruction!(
                Var::GetCoord, [(Ident, value_ident), (Ident, accesed_ident)], [
                    match coord {
                        CodegenLocationCoordinate::X => Tag::new(Tags::Var::GetCoord::Coordinate::X),
                        CodegenLocationCoordinate::Y => Tag::new(Tags::Var::GetCoord::Coordinate::Y),
                        CodegenLocationCoordinate::Z => Tag::new(Tags::Var::GetCoord::Coordinate::Z),
                        CodegenLocationCoordinate::Pitch => Tag::new(Tags::Var::GetCoord::Coordinate::Pitch),
                        CodegenLocationCoordinate::Yaw => Tag::new(Tags::Var::GetCoord::Coordinate::Yaw),
                    }
                ]
            ),
            CodegenTraceCrumbIdent::Vector(coord) => instruction!(
                Var::GetVectorComp, [(Ident, value_ident), (Ident, accesed_ident)], [
                    match coord {
                        CodegenVectorCoordinate::X => Tag::new(Tags::Var::GetVectorComp::Component::X),
                        CodegenVectorCoordinate::Y => Tag::new(Tags::Var::GetVectorComp::Component::Y),
                        CodegenVectorCoordinate::Z => Tag::new(Tags::Var::GetVectorComp::Component::Z),
                    }
                ]
            ),
        });
    }

    fn set_crumb(&mut self, crumb: CodegenTraceCrumbIdent, value_ident: u32, accessed_ident: u32) {
        self.buffer.code_buffer.push_instruction(match crumb {
            CodegenTraceCrumbIdent::IndexDirect(index) => instruction!(
                Var::SetListValue, [(Ident, accessed_ident), (Int, index), (Ident, value_ident)]
            ),
            CodegenTraceCrumbIdent::Index(ident) => instruction!(
                Var::SetListValue, [(Ident, accessed_ident), (Ident, ident), (Ident, value_ident)]
            ),
            CodegenTraceCrumbIdent::Entry(ident) => instruction!(
                Var::SetDictValue, [(Ident, accessed_ident), (Ident, ident), (Ident, value_ident)]
            ),
            CodegenTraceCrumbIdent::Location(coord) => instruction!(
                Var::SetCoord, [(Ident, accessed_ident), (Ident, accessed_ident), (Ident, value_ident)], [
                    match coord {
                        CodegenLocationCoordinate::X => Tag::new(Tags::Var::SetCoord::Coordinate::X),
                        CodegenLocationCoordinate::Y => Tag::new(Tags::Var::SetCoord::Coordinate::Y),
                        CodegenLocationCoordinate::Z => Tag::new(Tags::Var::SetCoord::Coordinate::Z),
                        CodegenLocationCoordinate::Pitch => Tag::new(Tags::Var::SetCoord::Coordinate::Pitch),
                        CodegenLocationCoordinate::Yaw => Tag::new(Tags::Var::SetCoord::Coordinate::Yaw),
                    }
                ]
            ),
            CodegenTraceCrumbIdent::Vector(coord) => instruction!(
                Var::SetVectorComp, [(Ident, accessed_ident), (Ident, accessed_ident), (Ident, value_ident)], [
                    match coord {
                        CodegenVectorCoordinate::X => Tag::new(Tags::Var::SetVectorComp::Component::X),
                        CodegenVectorCoordinate::Y => Tag::new(Tags::Var::SetVectorComp::Component::Y),
                        CodegenVectorCoordinate::Z => Tag::new(Tags::Var::SetVectorComp::Component::Z),
                    }
                ]
            ),
        });
    }

    // Used internally by the ``generate_expression`` and ``generate_expression_inside`` functions, to generate the FINAL register in which
    // the resulting calculation of *that* branch (not the overall expression) is stored.
    fn generate_expression_allocate_register(&mut self, settings: &GenerateExpressionSettings, register_group: u64) -> u32 {
        if settings.generate_codeblocks == false {
            return self.buffer.constant_void();
        }
        if settings.depth == 0 {
            if let Some(set_ident) = settings.set_ident {
                return set_ident;
            }
            if let Some(outside_group) = settings.register_group {
                return self.buffer.allocate_grouped_line_register(outside_group);
            }
            // Unrecommended case here: allocates and returns an orphaned register
            return self.buffer.allocate_line_register();
        }
        return self.buffer.allocate_grouped_line_register(register_group);
    }

    /// Used internally by the ``generate_expression`` and ``generate_expression_inside`` functions to push instructions to the code buffer.
    fn push_expression_instruction(&mut self, settings: &GenerateExpressionSettings, instruction: Instruction) {
        if settings.generate_codeblocks {
            self.buffer.code_buffer.push_instruction(instruction);
        }
    }
    /// Used internally by the ``generate_expression`` and ``generate_expression_inside`` functions to push parameters to the code buffer.
    fn push_expression_parameter(&mut self, settings: &GenerateExpressionSettings, parameter: Parameter) {
        if settings.generate_codeblocks {
            self.buffer.code_buffer.push_parameter(parameter);
        }
    }

    fn implicitly_cast(&mut self, context: usize, root_node: &Rc<Node>, mut result: CodegenExpressionResult, settings: &GenerateExpressionSettings, register_group: u64) -> Result<CodegenExpressionResult, CodegenError> {
        let Some(value_type) = settings.expected_type.clone() else {
            return Ok(result);
        };
        if value_type == result.value.value_type {
            return Ok(result);
        }
        match (&result.value.value_type, &value_type) { //variable we have vs variable we want (num -> string, vec -> location, etc)
            (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::String)) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                self.push_expression_instruction(&settings, instruction!(
                    Var::String, [(Ident, register), (Ident, result.value.ident)]
                ));
                result.value.ident = register;
                result.value.value_type = value_type.clone();
            }
            (ValueType::Primitive(PrimitiveType::Vector), ValueType::Primitive(PrimitiveType::Location)) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                let location = self.buffer.use_location(ParameterValue::Int(0), ParameterValue::Int(0), ParameterValue::Int(0), ParameterValue::Int(0), ParameterValue::Int(0));
                self.push_expression_instruction(&settings, instruction!(
                    Var::ShiftOnVector, [(Ident, register), (Ident, location), (Ident, result.value.ident)]
                ));
                result.value.ident = register;
                result.value.value_type = value_type.clone();
            }
            _ => { return CodegenError::err(root_node.clone(), ErrorRepr::CantImplicitlyCast); }
        };

        return Ok(result);
    }

    fn generate_expression(&mut self, context: usize, root_node: &Rc<Node>, settings: GenerateExpressionSettings) -> Result<CodegenExpressionResult, CodegenError> {
        let mut expression_stack= VecDeque::new();
        expression_stack.push_back(CodegenExpressionStack::Node(root_node));
        let register_group = self.buffer.allocate_line_register_group();
        let result = self.generate_expression_inside(context, root_node, settings.clone(), register_group)?;
        self.buffer.free_line_register_group(register_group);
        Ok(result)
    }
    

    fn generate_expression_inside(&mut self, context: usize, node: &Rc<Node>, settings: GenerateExpressionSettings, register_group: u64) -> Result<CodegenExpressionResult, CodegenError> {
        let mut value = CodegenValue::default();
        let mut trace = None;
        match node.as_ref() {
            Node::Primary(token) => {
                let set_value = settings.depth == 0 && settings.variable_necessary; // if the depth is 0, that means this is the ONLY thing in the expression, hence we need to set the final variable.
                value = match &token.token_type {
                    TokenType::Ident(_ident) => {
                        // Struct Function `self` check.
                        let parent_context = self.parents[context];
                        if matches!(self.get_context_type(parent_context)?, ContextType::Struct) {
                            // This might be a stupid workaround or a genius one, but i am creating a fake node to pretend self is being accessed.
                            // This can be explained as when you type ``hp`` inside a Player { num hp; } struct, it first checks if you mean ``self.hp`` then does the other normal things.
                            let fake_access_node = Rc::new(Node::Access(
                                Rc::new(Node::Primary(
                                    Rc::new(Token {
                                        token_type: TokenType::Keyword(Keyword::Value(ValuedKeyword::SelfIdentity)),
                                        range: Range::new((0,0), (0,0))}
                                    )
                                )),     
                                node.clone()
                            ));

                            if let Ok(result) = self.generate_expression(context, &fake_access_node, settings.clone()) {
                                return Ok(result);
                            }
                        }

                        let get_var = self.get_definition_access_isolated(context, &settings, node, register_group)?;
                        trace = get_var.trace;
                        // Possibly here, there might be something that necessarily sets 'set_value' to false. I haven't found an edge case like that,
                        // but even if there is one it's just an extra unnecessary codeblock.
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
                    TokenType::Keyword(Keyword::Value(keyword)) => {
                        match keyword {
                            ValuedKeyword::SelfIdentity => {
                                let parent_context = self.parents[context];
                                let ContextType::Struct = self.get_context_type(parent_context)? else {
                                    return CodegenError::err(node.clone(), ErrorRepr::SelfInObjectiveCode);
                                };
                                let self_variable = self.buffer.use_variable("self", DP::Var::Scope::Line);
                                trace = Some(CodegenTrace::root(self_variable));
                                CodegenValue::new(
                                    self_variable,
                                    ValueType::Struct(parent_context)
                                )
                            }
                            _ => { return CodegenError::err(node.clone(), ErrorRepr::UnexpectedPrimaryToken); }
                        }
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::UnexpectedPrimaryToken); }
                };
                if set_value {
                    let register = self.generate_expression_allocate_register(&settings, register_group);
                    self.push_expression_instruction(&settings, instruction!(
                        Var::Set, [(Ident, register), (Ident, value.ident)]
                    ))
                }
            }
            Node::Vector(xn, yn, zn) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                let temp_group = self.buffer.allocate_line_register_group();
                let x = self.generate_expression_inside(context, xn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                let y = self.generate_expression_inside(context, yn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                let z = self.generate_expression_inside(context, zn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                value.ident = register;
                value.value_type = ValueType::Primitive(PrimitiveType::Vector);
                self.push_expression_instruction(&settings, instruction!(
                    Var::Vector, [ (Ident, register), (Ident, x.ident), (Ident, y.ident), (Ident, z.ident) ]
                ));
                self.buffer.free_line_register_group(temp_group);
            }
            Node::Location(xn, yn, zn, pitchn, yawn) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                let temp_group = self.buffer.allocate_line_register_group();
                let x = self.generate_expression_inside(context, xn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                let y = self.generate_expression_inside(context, yn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                let z = self.generate_expression_inside(context, zn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                let pitch = self.generate_expression_inside(context, pitchn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                let yaw = self.generate_expression_inside(context, yawn, GenerateExpressionSettings::parameter(temp_group).expect_type(&ValueType::Primitive(PrimitiveType::Number)), register_group)?.value.clone();
                value.ident = register;
                value.value_type = ValueType::Primitive(PrimitiveType::Location);
                self.push_expression_instruction(&settings, instruction!(
                    Var::SetAllCoords, [ (Ident, register), (Ident, x.ident), (Ident, y.ident), (Ident, z.ident), (Ident, pitch.ident), (Ident, yaw.ident) ]
                ));
                self.buffer.free_line_register_group(temp_group);
            }
            Node::Construct(construct_ident, construct_body) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                let created_struct = self.create_struct_instance_from_node(context, construct_ident, construct_body, register)?;
                value = CodegenValue::new(register, created_struct);
            }
            Node::Declaration(decl_type, decl_ident) => {
                value = self.declare_runtime_variable(context, decl_type, decl_ident)?.0.variable.clone();
                trace = Some(CodegenTrace::root(value.ident));
            }
            Node::Assignment(assign_var_node, assign_value_node) => {
                let assign_var = self.generate_expression_inside(context, assign_var_node, GenerateExpressionSettings::comptime(), register_group)?.clone();
                value = assign_var.value.clone();
                trace = assign_var.trace;
                //TODO: Hardcoded casting options (to cast vectors to locations and whatnot)
                let Some(trace_set) = trace.clone() else {
                    return CodegenError::err(assign_var_node.clone(), ErrorRepr::NoTraceCantAssign)
                };
                let assign_value = if trace_set.crumbs.len() == 0 { // No access variable, set_ident method
                    self.generate_expression_inside(context, assign_value_node, GenerateExpressionSettings::ident(value.ident).expect_type(&assign_var.value.value_type), register_group)?
                } else {
                    let param_group = self.buffer.allocate_line_register_group();
                    let assign_value = self.generate_expression_inside(context, assign_value_node, GenerateExpressionSettings::parameter(param_group).expect_type(&assign_var.value.value_type), register_group)?;
                    self.set_trace_to_value(context, trace_set.clone(), assign_value.value.clone())?;
                    self.buffer.free_line_register_group(param_group);
                    assign_value
                };
            }
            Node::FunctionCall(func_ident, func_params) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                let func_type = self.call_function(context, func_ident, func_params, register)?;
                value = CodegenValue::new(register, func_type);
            }
            Node::Access(accessed, access_field) => {
                let accessed_expression: CodegenExpressionResult = self.generate_expression_inside(context, accessed, settings.pass(), register_group)?.clone();
                let accessed_value = accessed_expression.value;
                let access_field_ident = Self::get_primary_as_ident(access_field, ErrorRepr::ExpectedAccessableIdentifier)?;

                
                match accessed_value.value_type {
                    ValueType::Struct(struct_id) => {
                        let register = self.generate_expression_allocate_register(&settings, register_group);
                        value.ident = register;
                        let struct_context = self.context_borrow(struct_id)?;

                        let definition = struct_context
                            .definition_lookup
                            .get(access_field_ident)
                            .ok_or(CodegenError::new(access_field.clone(), ErrorRepr::InvalidStructDefinition))?;
                    
                        if let Ok(field_id) = self.extract_definition_field(definition) {
                            let field_type = struct_context.fields[field_id].field_type.clone();
                            drop(struct_context);
                            let field_index = field_id + 1;
                            self.push_expression_instruction(&settings, instruction!(
                                Var::GetListValue, [ (Ident, register), (Ident, accessed_value.ident), (Int, field_index) ]
                            ));
                            if let Some(mut trace_add) = accessed_expression.trace {
                                trace_add.crumbs.push(CodegenTraceCrumb::Ident(CodegenTraceCrumbIdent::IndexDirect(field_index)));
                                trace = Some(trace_add);
                            }
    
                            value.value_type = field_type;
                        } else if let Ok(func_id) = self.extract_definition_function(definition) {
                            drop(struct_context);
                            return Ok(CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::SelfFunction(func_id, accessed_value.ident))))
                        } else {
                            panic!("Invalid struct access found which *is* correctly looked up but is neither a field nor a function.");
                        }
                    }
                    ValueType::Primitive(PrimitiveType::Location) => {
                        let register = self.generate_expression_allocate_register(&settings, register_group);
                        value.ident = register;
                        value.value_type = ValueType::Primitive(PrimitiveType::Number);
                        let crumb = CodegenTraceCrumbIdent::Location(match access_field_ident.as_str() {
                            "x" => CodegenLocationCoordinate::X,
                            "y" => CodegenLocationCoordinate::Y,
                            "z" => CodegenLocationCoordinate::Z,
                            "pitch" => CodegenLocationCoordinate::Pitch,
                            "yaw" => CodegenLocationCoordinate::Yaw,
                            _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidLocationAccess); }
                        });
                        if settings.generate_codeblocks {
                            self.get_crumb(crumb.clone(), register, accessed_value.ident);
                        }
                        if let Some(mut trace_add) = accessed_expression.trace {
                            trace_add.crumbs.push(CodegenTraceCrumb::Ident(crumb));
                            trace = Some(trace_add);
                        }
                    }
                    ValueType::Primitive(PrimitiveType::Vector) => {
                        let register = self.generate_expression_allocate_register(&settings, register_group);
                        value.ident = register;
                        value.value_type = ValueType::Primitive(PrimitiveType::Number);
                        let crumb = CodegenTraceCrumbIdent::Vector(match access_field_ident.as_str() {
                            "x" => CodegenVectorCoordinate::X,
                            "y" => CodegenVectorCoordinate::Y,
                            "z" => CodegenVectorCoordinate::Z,
                            _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidVectorAccess); }
                        });
                        if settings.generate_codeblocks {
                            self.get_crumb(crumb.clone(), register, accessed_value.ident);
                        }
                        if let Some(mut trace_add) = accessed_expression.trace {
                            trace_add.crumbs.push(CodegenTraceCrumb::Ident(crumb));
                            trace = Some(trace_add);
                        }
                    }
                    ValueType::Comptime(ComptimeType::Domain(domain_cid)) => {
                        let mut set_value = settings.depth == 0 && settings.variable_necessary;
                        let get_access = self.get_definition_access(domain_cid, &settings, access_field, register_group)?;
                        value = get_access.value;
                        if value.value_type.is_comptime() {
                            set_value = false;
                        }
                        trace = get_access.trace;
                        if set_value {
                            let register = self.generate_expression_allocate_register(&settings, register_group);
                            self.push_expression_instruction(&settings, instruction!(
                                Var::Set, [(Ident, register), (Ident, value.ident)]
                            ))
                        }
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion); }
                }
            }
            Node::ListCall(called, index_field) => {
                let called_expression = self.generate_expression_inside(context, called, settings.pass(), register_group)?.clone();
                let called_value = called_expression.value;

                
                match called_value.value_type {
                    ValueType::Primitive(PrimitiveType::List(inside_type)) => {
                        let register = self.generate_expression_allocate_register(&settings, register_group);
                        value.ident = register;

                        let index = self.generate_expression_inside(context, index_field, GenerateExpressionSettings::parameter(register_group).keep_comptime(&settings), register_group)?.value.clone();
                        if !matches!(index.value_type, ValueType::Primitive(PrimitiveType::Number)) {
                            return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion);
                        }
                        let index_register = self.buffer.allocate_grouped_line_register(register_group);
                        self.push_expression_instruction(&settings, instruction!(
                            Var::Add, [ (Ident, index_register), (Ident, index.ident), (Int, 1) ]
                        ));

                        self.push_expression_instruction(&settings, instruction!(
                            Var::GetListValue, [ (Ident, register), (Ident, called_value.ident), (Ident, index_register) ]
                        ));
                        if let Some(mut trace_add) = called_expression.trace {
                            trace_add.crumbs.push(CodegenTraceCrumb::IndexNode(index_field.clone()));
                            trace = Some(trace_add);
                        }

                        value.value_type = inside_type.as_ref().clone();
                    }
                    ValueType::Primitive(PrimitiveType::Map(mapped_type, mapping_type)) => {
                        let register = self.generate_expression_allocate_register(&settings, register_group);
                        value.ident = register;

                        let index = self.generate_expression_inside(context, index_field, GenerateExpressionSettings::parameter(register_group).keep_comptime(&settings), register_group)?.value.clone();
                        if &index.value_type != mapping_type.as_ref() {
                            return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion);
                        }
                        let register_index = if !matches!(index.value_type, ValueType::Primitive(PrimitiveType::String)) {
                            let temp_reg = self.buffer.allocate_grouped_line_register(register_group);
                            self.push_expression_instruction(&settings, instruction!(
                                Var::String, [ (Ident, temp_reg), (String, " "), (Ident, index.ident) ]
                            ));
                            temp_reg
                        } else {
                            index.ident
                        };

                        self.push_expression_instruction(&settings, instruction!(
                            Var::GetDictValue, [ (Ident, register), (Ident, called_value.ident), (Ident, register_index) ]
                        ));
                        if let Some(mut trace_add) = called_expression.trace {
                            trace_add.crumbs.push(CodegenTraceCrumb::EntryNode(index_field.clone()));
                            trace = Some(trace_add);
                        }

                        value.value_type = mapped_type.as_ref().clone();
                    }
                    ValueType::Comptime(ComptimeType::Type(called_type)) => {
                        let value_type = match index_field.as_ref() {
                            Node::None => RealtimeValueType::Primitive(PrimitiveType::List(Rc::new(called_type.normalize()))),
                            _ => RealtimeValueType::Primitive(PrimitiveType::Map(Rc::new(called_type.normalize()), Rc::new(self.get_type(index_field, context)?)))
                        };
                        return Ok(CodegenExpressionResult::value(CodegenValue::comptime(self.buffer.constant_void(), ComptimeType::Type(value_type))));
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion); }
                }
            }
            Node::List(tuple) => {
                let expected_list_type = 
                    settings.expected_type
                    .clone()
                    .unwrap_or(if tuple.len() > 0 {
                        ValueType::Primitive(PrimitiveType::List(Rc::new(self.generate_expression_inside(context, &tuple[0], GenerateExpressionSettings::comptime(), register_group)?.value.value_type)))
                    } else { 
                        ValueType::Primitive(PrimitiveType::List(Rc::new(ValueType::Primitive(PrimitiveType::None))))
                    });
                value.value_type = expected_list_type.clone();
                let ValueType::Primitive(PrimitiveType::List(expected_type)) = expected_list_type else {
                    return CodegenError::err(node.clone(), ErrorRepr::ExpectedListType);
                };
                let expected_type = expected_type.as_ref().clone();
                let register = self.generate_expression_allocate_register(&settings, register_group);
                value.ident = register;

                let mut values = Vec::new();
                for element in tuple {
                    values.push(self.generate_expression_inside(context, element, settings.pass().expect_type(&expected_type), register_group)?.value.clone());
                }
                self.push_expression_instruction(&settings,
                    instruction!( Var::CreateList, [ (Ident, register) ])
                );
                for (values, value) in values.iter().enumerate() {
                    if values % 26 == 0 && values > 0 {
                        self.push_expression_instruction(&settings, 
                            instruction!( Var::AppendList, [ (Ident, register) ])
                        );
                    }
                    self.push_expression_parameter(&settings, Parameter::from_ident(value.ident));
                }
            }
            Node::Sum(l, r) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                let l = self.generate_expression_inside(context, l, settings.pass(), register_group)?.value.clone();
                let r = self.generate_expression_inside(context, r, settings.pass(), register_group)?.value.clone();
                value.ident = register;
                match (l.value_type, r.value_type) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.push_expression_instruction(&settings, instruction!(
                            Var::Add, [ (Ident, register), (Ident, l.ident), (Ident, r.ident) ]
                        ));
                        value.value_type = ValueType::Primitive(PrimitiveType::Number);
                    }
                    (ValueType::Primitive(PrimitiveType::String), ValueType::Primitive(PrimitiveType::String) | ValueType::Primitive(PrimitiveType::Number)) => {
                        self.push_expression_instruction(&settings, instruction!(
                            Var::String, [ (Ident, register), (Ident, l.ident), (Ident, r.ident) ]
                        ));
                        value.value_type = ValueType::Primitive(PrimitiveType::String);
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion); }
                }
            }
            Node::DFASM(params, return_type, block) => {
                let dfasm_group = self.buffer.allocate_line_register_group();
                let register = self.generate_expression_allocate_register(&settings, register_group);
                value.ident = register;
                
                let return_type = if let Node::None = return_type.as_ref() {
                    settings.expected_type.clone().unwrap_or(ValueType::Primitive(PrimitiveType::None))
                } else {
                    self.get_type(return_type, context)?
                };
                let Node::Primary(block_token) = block.as_ref() else {
                    return CodegenError::err(block.clone(), ErrorRepr::ExpectedBlock);
                };
                let TokenType::DFASM(dfasm_str) = block_token.as_ref().token_type.clone() else {
                    return CodegenError::err(block.clone(), ErrorRepr::ExpectedBlock);
                };
                let mut compiler = Compiler::new(dfasm_str.as_str());
                compiler.identifier_count = self.buffer.ident_count;
                for (param_id, param) in Self::extract_parameter_vec(params)?.iter().enumerate() {
                    let param_value = self.generate_expression_inside(context, param, settings.pass(), dfasm_group)?.value.clone();
                    compiler.references.insert(param_id.to_string(), param_value.ident);
                }
                compiler.references.insert("".to_owned(), register);
                let added_identifiers = compiler.identifier_count;
                compiler.compile_string().map_err(|_| CodegenError::new(block.clone(), ErrorRepr::DFASMError))?;
                let added_identifiers = compiler.identifier_count - added_identifiers;
                self.buffer.ident_count += added_identifiers;
                self.buffer.code_buffer.append_bin_mut(&mut compiler.bin);
                value.ident = register;
                value.value_type = return_type;
                self.buffer.free_line_register_group(dfasm_group);
            }
            Node::Product(l, r) => {
                let register = self.generate_expression_allocate_register(&settings, register_group);
                let l = self.generate_expression_inside(context, l, settings.pass(), register_group)?.value.clone();
                let r = self.generate_expression_inside(context, r, settings.pass(), register_group)?.value.clone();
                value.ident = register;
                match (l.value_type, r.value_type) {
                    (ValueType::Primitive(PrimitiveType::Number), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.push_expression_instruction(&settings, instruction!(
                            Var::Mul, [ (Ident, register), (Ident, l.ident), (Ident, r.ident) ]
                        ));
                        value.value_type = ValueType::Primitive(PrimitiveType::Number);
                    }
                    (ValueType::Primitive(PrimitiveType::String), ValueType::Primitive(PrimitiveType::Number)) => {
                        self.push_expression_instruction(&settings, instruction!(
                            Var::RepeatString, [ (Ident, register), (Ident, l.ident), (Ident, r.ident) ]
                        ));
                        value.value_type = ValueType::Primitive(PrimitiveType::String);
                    }
                    _ => { return CodegenError::err(node.clone(), ErrorRepr::InvalidExpressionTypeConversion); }
                }
            }
            _ => { return CodegenError::err(node.clone(), ErrorRepr::UnexpectedExpressionToken); }
        }
        let result = CodegenExpressionResult {
            value,
            trace
        };
        let result = self.implicitly_cast(context, node, result, &settings, register_group)?;
        Ok(result)
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
        self.block_runtime_vars_add.push(runtime_str.clone());
        Ok((self.runtime_vars[context].get(&runtime_str).unwrap(), runtime_str))
    }

    fn call_function(&mut self, context: usize, function_ident: &Rc<Node>, function_params: &Rc<Node>, return_ident: u32) -> Result<ValueType, CodegenError> {
        // let func_context = self.find_function_by_node(function_ident, context)?;
        let function_ident_evaluation = self.generate_expression(context, function_ident, GenerateExpressionSettings::comptime().prefer_category(IdentifierCategory::Function))?;
        let (func_context, struct_func_ident) = match function_ident_evaluation.value.value_type {
            ValueType::Comptime(ComptimeType::Function(func_context)) => (func_context, None),
            ValueType::Comptime(ComptimeType::SelfFunction(func_context, struct_func_ident)) => (func_context, Some(struct_func_ident)),
            _ => { return CodegenError::err(function_ident.clone(), ErrorRepr::ExpectedFunctionIdentifier); }
        };
        let func_name = self.get_context_full_name(func_context).clone();
        let func_id = self.buffer.use_function(func_name.as_str());
        let mut call_instruction = instruction!(Call, [
            (Ident, func_id)
        ]);
        let ContextType::Function(ret_type_field) = self.context_borrow(func_context)?.context_type.clone() else {
            return CodegenError::err_headless(ErrorRepr::Generic);
        };
        if let Some(struct_func_ident) = struct_func_ident {
            call_instruction.params.push(Parameter::from_ident(struct_func_ident));
        }
        if !matches!(ret_type_field, ValueType::Primitive(PrimitiveType::None)) { // function has a return value
            call_instruction.params.push(Parameter::from_ident(return_ident))
        }
        let params = Self::extract_parameter_vec(function_params)?;
        let param_register_group = self.buffer.allocate_line_register_group();
        let func_fields = self.context_borrow(func_context)?.fields.clone();
        if params.len() > func_fields.len() {
            return CodegenError::err(function_params.clone(), ErrorRepr::UnexpectedFunctionParameter)
        }
        if params.len() < func_fields.len() {
            return CodegenError::err(function_params.clone(), ErrorRepr::ExpectedFunctionParameter)
        }
        for (param, param_field) in params.into_iter().zip(func_fields) {
            let param_expression = self.generate_expression(context, &param, GenerateExpressionSettings::parameter(param_register_group).expect_type(&param_field.field_type))?;
            call_instruction.params.push(Parameter::from_ident(param_expression.value.ident));
        }

        self.buffer.code_buffer.push_instruction(call_instruction);
        self.buffer.free_line_register_group(param_register_group);
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
        if matches!(parent_context, ContextType::Struct) {
            let self_struct_param_ident = self.buffer.use_return_param("self");
            self.buffer.code_buffer.push_parameter(Parameter::from_ident(self_struct_param_ident.0));
        }
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
            match statement.as_ref() {
                Node::Declaration(..) | Node::Assignment(..) | Node::DFASM(..) => {
                    let void_register = self.buffer.constant_void();
                    self.generate_expression(context, &statement.clone(), GenerateExpressionSettings::void(void_register))?;
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
                    let if_allocation = self.buffer.allocate_line_register_group();
                    let expr_id = self.generate_expression(context, if_condition, GenerateExpressionSettings::parameter(if_allocation).expect_type(&ValueType::Primitive(PrimitiveType::Bool)))?;
                    self.buffer.code_buffer.push_instruction(instruction!(
                        Varif::Eq, [
                            (Ident, expr_id.value.ident),
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
                    self.buffer.free_line_register_group(if_allocation);
                },
                Node::Return(return_value) => {
                    if !matches!(return_value.as_ref(), Node::None) { //You're returning a value
                        let Some(return_type_ident_some) = return_type_ident else {
                            return CodegenError::err(return_value.clone(), ErrorRepr::UnexpectedReturnValue)
                        };
                        self.generate_expression(context, return_value, GenerateExpressionSettings::ident(return_type_ident_some).expect_type(&return_type))?;
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
                    let while_allocation = self.buffer.allocate_line_register_group();
                    let expr_id = self.generate_expression(context, while_cond, GenerateExpressionSettings::parameter(while_allocation).expect_type(&ValueType::Primitive(PrimitiveType::Bool)))?;
                    self.buffer.code_buffer.push_instruction(instruction!(
                        Varif::Eq, [
                            (Ident, expr_id.value.ident),
                            (Int, 1)
                        ]
                    ));
                    self.buffer.code_buffer.push_instruction(instruction!(Ctrl::StopRepeat));
                    self.buffer.code_buffer.push_instruction(instruction!(EndIf));
                    let Node::Block(if_block) = while_block.as_ref() else {
                        return CodegenError::err(while_block.clone(), ErrorRepr::ExpectedBlock);
                    };
                    body_stack.push_front((0, Rc::new(if_block.clone()), Vec::new(), Some(instruction!(EndRep)), body_stack_mode));
                    self.buffer.free_line_register_group(while_allocation);
                }
                _ => {
                    println!("Unrecognized Statement: {:#?}", statement.clone());
                }
            }
            body_stack[0].2.extend(self.block_runtime_vars_add.clone());
            self.block_runtime_vars_add.clear();
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

    use esh_parser::parser::*;
    use lexer::{Lexer, types::Token};
    use super::*;

    #[test]
    pub fn decompile_from_file_test() {
        let name = "practical_dfasm";
        // let path = r"C:\Users\koren\OneDrive\Documents\Github\Esh\codegen\examples\";
        let path = r"K:\Programming\GitHub\Esh\codegen\examples\";

        let file_bytes = fs::read(format!("{}{}.esh", path, name)).expect("File should read");
        let lexer = Lexer::new(str::from_utf8(&file_bytes).expect("Should encode to utf-8"));
        let lexer_tokens: Vec<Rc<Token>> = lexer.map(|v| Rc::new(v.expect("Lexer token should unwrap"))).collect();
        
        println!("LEXER TOKENS\n----------------------\n{:#?}\n----------------------", lexer_tokens);
        let mut parser = Parser::new(lexer_tokens.as_slice());
        let parser_tree = Rc::new(parser.parse().expect("Parser statement block should unwrap"));
        //##println!("PARSER TREE\n----------------------\n{:#?}\n----------------------", parser_tree);
        
        let mut codegen = CodeGen::new();
        codegen.codegen_from_node(parser_tree.clone()).expect("Codegen should generate");
        // println!("CODEGEN CONTEXTS\n----------------------\n{:#?}\n----------------------", codegen.contexts);

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