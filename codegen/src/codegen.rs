use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use dfbin::enums::{Parameter, ParameterValue};
use dfbin::instruction;
use dfbin::Constants::Tags::DP;
use lexer::types::TokenType;
use parser::parser::Node;
use crate::buffer::CodeGenBuffer;
use crate::errors::{CodegenError, ErrorRepr};
use crate::context::{CodeDefinition, CodeScope, Context, ContextType};
use crate::types::{CodegenExpressionStack, CodegenExpressionType, Field, FieldModifier, FieldType, PrimitiveType, RuntimeVariable};

pub struct CodeGen {
    pub context_map: HashMap<String, usize>,
    pub root_context: usize,
    pub contexts: Vec<Rc<RefCell<Context>>>,

    current_id: usize,
    buffer: CodeGenBuffer,
    parents: Vec<usize>,
    runtime_vars: Vec<HashMap<String, RuntimeVariable>>,
    return_runtimes: Vec<Option<RuntimeVariable>>,
    function_idents: Vec<u32>,
    field_names: Vec<Vec<String>>,
    context_names: Vec<String>,
    context_full_names: Vec<String>,
    run: usize,
}

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
            function_idents: Vec::new(),
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
        println!("{:?}, {:?}", current_id, depth);
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
            str.push_str(&context_name);
            str
        } else {
            let mut str = self.context_full_names[parent_id].clone();
            str.push('.');
            str.push_str(&context_name);
            str
        });
        let mut current_context = current_context_cell.borrow_mut();
        self.current_id += 1;
        let mut body = Vec::new();
        for node in node_block {
            match (node.as_ref(), &context_type) {
                (Node::Func(ident, params, return_type, body), _) => {
                    let return_type_field = FieldType::Ident(return_type.clone());
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
                            field_type: FieldType::Ident(param_type.clone()),
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
                        field_type: FieldType::Ident(field_type.clone()),
                        scope: CodeScope::Public,
                    })
                }
                (_, ContextType::Struct) => {
                    return CodegenError::err(node.clone(), ErrorRepr::UnstructuredStructCode);
                }
                _ => {
                    body.push(node.clone());
                }
            };
        };
        println!("WOWWWWW {:?}\n{:?}\n\n", current_id, field_names);
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

    fn generate_all_code(&mut self) -> Result<(), CodegenError> {
        for context_id in 0..self.contexts.len() {
            self.generate_code(context_id)?;
        }
        Ok(())
    }

    fn generate_code(&mut self, context: usize) -> Result<(), CodegenError> {
        let (body, context_type, fields) = {
            let context_borrow = self.context_borrow(context)?;
            println!("\nGenerating for {:?}\nContext Type: {:?}\n\n", context_borrow.id, context_borrow.context_type);
            (context_borrow.body.clone(), context_borrow.context_type.clone(), context_borrow.fields.clone())
        };
        match context_type {
            ContextType::Struct => {

            },
            ContextType::Function(return_type) => {
                self.generate_function_code(context, body, fields, return_type)?;
            },
            ContextType::Namespace => {

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

    fn get_context_name(&self, context: usize) -> &String {
        &self.context_names[context]
    }

    fn get_context_full_name(&self, context: usize) -> &String {
        &self.context_full_names[context]
    }

    fn generate_expression(&mut self, context: usize, root_node: &Rc<Node>, set_ident: u32) -> Result<(u32, FieldType), CodegenError> {
        println!("{:#?}", root_node);
        let mut expression_stack= VecDeque::new();
        expression_stack.push_back(CodegenExpressionStack::Node(root_node));
        let mut ident_stack = Vec::new();
        let mut ind_counter = 0;
        let mut register_id = 0;
        while expression_stack.len() > 0 {
            let get_expr = expression_stack.pop_front().expect("This should pop since len > 0");
            println!("\n\n#{} (Ident Stack: {:?})\n----------------\n{:?}", ind_counter, ident_stack, get_expr);
            match get_expr {
                CodegenExpressionStack::Node(node) => {
                    let matches = matches!(node.as_ref(), Node::Primary(..));
                    let ident = if register_id == 0 || matches {
                        set_ident
                    } else {
                        self.buffer.use_line_register(register_id)
                    };
                    match node.as_ref() {
                        Node::Primary(token) => {
                            let (ident, ident_type) = match &token.as_ref().token_type {
                                TokenType::Ident(..) => {
                                    let v = self.find_variable_by_name(context, node)?;
                                    (v.ident, v.field_type.clone())
                                },
                                TokenType::String(t) => (self.buffer.use_string(t.as_str()), FieldType::Primitive(PrimitiveType::String)),
                                TokenType::Number(t) => (self.buffer.use_number(ParameterValue::Float(*t)), FieldType::Primitive(PrimitiveType::Number)),
                                _ => {
                                    return CodegenError::err(root_node.clone(), ErrorRepr::UnexpectedExpressionToken)
                                }
                            };
                            ident_stack.push((ident, ident_type));
                        },
                        Node::Sum(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Add,
                                (register_id, ident), //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        Node::Difference(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Sub,
                                (register_id, ident), //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        
                        Node::Product(nl, nr) => {
                            expression_stack.push_front(CodegenExpressionStack::Calculate(
                                CodegenExpressionType::Mul,
                                (register_id, ident), //Register ID & Identifier
                                2 //How many elements we'll be expecting
                            ));
                            expression_stack.push_front(CodegenExpressionStack::Node(nl));
                            expression_stack.push_front(CodegenExpressionStack::Node(nr));
                        },
                        _ => {}
                    }
                    if !matches {
                        register_id += 1;
                    }
                }
                CodegenExpressionStack::Calculate(expression_type, register_and_ident, elms) => {
                    let mut parameters = Vec::new();
                    let mut types = Vec::new();
                    for _i in 0..elms {
                        let ident_from_stack = ident_stack.pop().expect("Ident stack should pop when calculating.");
                        parameters.push(Parameter{
                            value: ParameterValue::Ident(ident_from_stack.0),
                            slot: None,
                        });
                        types.push(ident_from_stack.1)
                    }
                    let result_type = self.calculate_expression(expression_type, register_and_ident, parameters, types, root_node)?;
                    ident_stack.push((self.buffer.use_line_register(register_and_ident.0), result_type));
                }
                
            }
            ind_counter += 1;
        }
        Ok(ident_stack.pop().expect("Ident stack should pop final value."))
    }

    fn calculate_expression(&mut self, expression_type: CodegenExpressionType, (_register, ident): (usize, u32), parameters: Vec<Parameter>, types: Vec<FieldType>, root_node: &Rc<Node>) -> Result<FieldType, CodegenError> {
        let result_type = match expression_type {
            CodegenExpressionType::Add => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (FieldType::Primitive(PrimitiveType::String), FieldType::Primitive(PrimitiveType::String)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::String, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        FieldType::Primitive(PrimitiveType::String)
                    },
                    (FieldType::Primitive(PrimitiveType::Number), FieldType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Add, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        FieldType::Primitive(PrimitiveType::Number)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Sub => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (FieldType::Primitive(PrimitiveType::Number), FieldType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Sub, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        FieldType::Primitive(PrimitiveType::Number)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Mul => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (FieldType::Primitive(PrimitiveType::Number), FieldType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Mul, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        FieldType::Primitive(PrimitiveType::Number)
                    },
                    (FieldType::Primitive(PrimitiveType::String), FieldType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::RepeatString, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        FieldType::Primitive(PrimitiveType::String)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
            CodegenExpressionType::Div => {
                match (types.get(0).unwrap(), types.get(1).unwrap()) {
                    (FieldType::Primitive(PrimitiveType::Number), FieldType::Primitive(PrimitiveType::Number)) => {
                        self.buffer.code_buffer.push_instruction(instruction!(
                            Var::Div, [
                                (Ident, ident)
                            ]
                        ));
                        self.buffer.code_buffer.push_parameter(parameters[0].clone());
                        self.buffer.code_buffer.push_parameter(parameters[1].clone());
                        FieldType::Primitive(PrimitiveType::Number)
                    },
                    _ => {
                        return CodegenError::err(root_node.clone(), ErrorRepr::InvalidExpressionTypeConversion)
                    }
                }
            },
        };
        Ok(result_type)
    }

    fn generate_function_code(&mut self, context: usize, body: Rc<Vec<Rc<Node>>>, fields: Vec<Field>, return_type: FieldType) -> Result<(), CodegenError> {
        // self.return_runtimes[context] = 
        let func_name = self.get_context_full_name(context).clone();
        let func_id = self.buffer.use_function(func_name.as_str());
        self.buffer.code_buffer.push_instruction(instruction!(
            Func, [
                (Ident, func_id)
            ]
        ));
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
            self.buffer.code_buffer.push_parameter(Parameter{
                value: ParameterValue::Ident(param_and_var_ident.0),
                slot: None
            });
            field_id += 1;
        }
        
        for statement in body.as_ref() {
            match statement.as_ref() {
                Node::Declaration(decl_type, decl_ident) => {
                    let decl_ident = Self::get_primary_as_ident(decl_ident, ErrorRepr::ExpectedVariableIdentifier)?;
                    let decl_type = self.find_type_by_ident(decl_type, context)?;
                    let var_name = Self::make_var_name(decl_ident, "rvl");
                    let var_ident = self.buffer.use_variable(var_name.as_ref(), DP::Var::Scope::Line);
                    let runtime_str = decl_ident.to_owned();
                    self.runtime_vars[context].insert(
                        runtime_str, 
                        RuntimeVariable::new(
                            decl_type,
                            var_name,
                            var_ident
                        )
                    );
                },
                Node::Assignment(assign_var, assign_value) => {
                    let (runtime_var_ident, runtime_var_field_type) = {
                        let runtime_var = self.find_variable_by_name(context, assign_var)?;
                        (runtime_var.ident, runtime_var.field_type.clone())
                    };
                    let expr_id = self.generate_expression(context, assign_value, runtime_var_ident)?;
                    if expr_id.1 != runtime_var_field_type {
                        return CodegenError::err(statement.clone(), ErrorRepr::InvalidVariableType)
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    pub fn codegen_from_node(&mut self, node: Rc<Node>) -> Result<(), CodegenError> {
        let _root_context = self.scan_block_outline(node, ContextType::Namespace, 0, 0, CodeScope::Public, Vec::new(), "main".to_owned())?;
        self.fill_all_field_types()?;
        self.root_context = 0;
        self.buffer.clear();
        println!("\n\n\n\n{:#?}\n\n\n\n", self.context_names);
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
        let name = "basic";
        let path = r"C:\Users\koren\OneDrive\Documents\Github\Esh\codegen\examples\";

        let file_bytes = fs::read(format!("{}{}.esh", path, name)).expect("File should read");
        let lexer = Lexer::new(str::from_utf8(&file_bytes).expect("Should encode to utf-8"));
        let lexer_tokens: Vec<Rc<Token>> = lexer.map(|v| Rc::new(v.expect("Lexer token should unwrap"))).collect();
        println!("LEXER TOKENS\n----------------------\n{:#?}\n----------------------", lexer_tokens);
        let mut parser = Parser::new(lexer_tokens.as_slice());
        let parser_tree = Rc::new(parser.parse().expect("Parser statement block should unwrap"));
        println!("PARSER TREE\n----------------------\n{:#?}\n----------------------", parser_tree);

        let mut codegen = CodeGen::new();
        codegen.codegen_from_node(parser_tree.clone()).expect("Codegen should generate");
        println!("CODEGEN CONTEXTS\n----------------------\n{:#?}\n----------------------", codegen.contexts);
        let code = codegen.buffer.flush();
        code.write_to_file(&format!("{}{}.dfbin", path, name)).expect("DFBin should write");
        let mut decompiler = decompiler::Decompiler::new(code).expect("Decompiler should create");
        decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::camelCase);
        let decompiled = decompiler.decompile().expect("Decompiler should decompile");
        println!("DECOMPILED\n----------------------\n{}\n----------------------", decompiled);
        fs::write(format!("{}{}.dfa", path, name), decompiled).expect("Decompiled DFA should write.");
    }

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
        codegen.codegen_from_node(parser_tree.clone()).expect("Codegen should generate");
        println!("CODEGEN CONTEXTS\n----------------------\n{:#?}\n----------------------", codegen.contexts);
    }
}