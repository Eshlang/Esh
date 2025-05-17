use std::collections::HashMap;
use dfbin::{enums::{Parameter, ParameterValue, Tag}, instruction, Constants::{self, Tags::DP::{Loc::{Pitch, Yaw}, Var::Scope}}, DFBin};
use Constants::Tags::DP;

use crate::{constants::CodeGenConstants, errors::{CodegenError, ErrorRepr}};

pub struct CodeGenBuffer {
    pub constants: CodeGenConstants,
    pub code_buffer: DFBin,
    pub func_buffer: DFBin,
    pub param_buffer: DFBin,
    pub(crate) ident_count: u32,
    idents_number_hash: HashMap<String, u32>,
    idents_location_hash: HashMap<String, u32>,
    idents_string_hash: HashMap<String, u32>,
    idents_variable_hash: HashMap<String, u32>,
    idents_param_hash: HashMap<String, u32>,
    idents_return_param_hash: HashMap<String, u32>,
    idents_function_hash: HashMap<String, u32>,

    line_register_idents: HashMap<usize, u32>,
    line_register_indices: HashMap<u32, usize>,
    
    allocated_line_registers: Vec<u64>,
    allocated_line_register_groups: HashMap<u64, Vec<u32>>,
    line_register_groups: u64
}

impl CodeGenBuffer {
    pub fn new() -> CodeGenBuffer {
        Self {
            constants: CodeGenConstants::new(),
            code_buffer: DFBin::new(),
            func_buffer: DFBin::new(),
            param_buffer: DFBin::new(),
            ident_count: 0,
            idents_number_hash: HashMap::new(),
            idents_location_hash: HashMap::new(),
            idents_string_hash: HashMap::new(),
            idents_variable_hash: HashMap::new(),
            idents_param_hash: HashMap::new(),
            idents_return_param_hash: HashMap::new(),
            idents_function_hash: HashMap::new(),
            
            line_register_idents: HashMap::new(),
            line_register_indices: HashMap::new(),

            allocated_line_registers: Vec::new(),
            allocated_line_register_groups: HashMap::new(),
            line_register_groups: 0
        }
    }
    fn clear_variables(&mut self) {
        self.ident_count = 0;
        self.idents_variable_hash.clear();
        self.idents_param_hash.clear();
        self.idents_number_hash.clear();
        self.idents_location_hash.clear();
        self.idents_function_hash.clear();
        self.idents_return_param_hash.clear();
        self.idents_string_hash.clear();

        self.line_register_idents.clear();
        self.line_register_indices.clear();
        self.allocated_line_registers.clear();
        self.allocated_line_register_groups.clear();

        self.line_register_groups = 0;

    }
    pub fn clear(&mut self) {
        self.code_buffer.clear();
        self.func_buffer.clear();
        self.param_buffer.clear();
        self.clear_variables();
    }
    pub fn flush(&mut self) -> DFBin {
        let mut flushed = DFBin::new();
        flushed.push_instruction(instruction!(Seg::Func));
        flushed.append_bin_mut(&mut self.func_buffer);
        flushed.push_instruction(instruction!(Seg::Param));
        flushed.append_bin_mut(&mut self.param_buffer);
        flushed.push_instruction(instruction!(Seg::Code));
        flushed.append_bin_mut(&mut self.code_buffer);
        self.clear_variables();
        flushed
    }

    fn _get_key(params: Vec<ParameterValue>) -> String {
        let mut total = String::new();
        for param in params {
            total.push_str(&Self::get_param_key(param));
        }
        total
    }

    fn get_param_key(param: ParameterValue) -> String {
        match param {
            ParameterValue::Ident(v) => {
                let mut v = v.to_string();
                v.push('v');
                v
            },
            ParameterValue::String(mut v) => {
                v.push('s');
                v
            },
            ParameterValue::Int(v) => {
                let mut v = v.to_string();
                v.push('i');
                v
            },
            ParameterValue::Float(v) => {
                let mut v = v.to_string();
                v.push('f');
                v
            },
        }
    }

    pub fn use_number(&mut self, number: ParameterValue) -> u32 {
        let key = Self::get_param_key(number.clone());
        if let Some(id) = self.idents_number_hash.get(&key) {
            return *id;
        }
        let param_id = self.ident_count;
        self.ident_count += 1;
        self.param_buffer.push_instruction(instruction!(
            DP::Num,
            [(Ident, param_id)]
        ));
        self.param_buffer.push_parameter(Parameter {
            value: number,
            slot: None
        });
        self.idents_number_hash.insert(key.to_owned(), param_id);
        param_id
    }

    pub fn use_location(&mut self, x: ParameterValue, y: ParameterValue, z: ParameterValue, pitch: ParameterValue, yaw: ParameterValue) -> u32 {
        let key = format!("{} {} {} {} {}", Self::get_param_key(x.clone()), Self::get_param_key(y.clone()), Self::get_param_key(z.clone()), Self::get_param_key(pitch.clone()), Self::get_param_key(yaw.clone()));
        if let Some(id) = self.idents_location_hash.get(&key) {
            return *id;
        }
        let param_id = self.ident_count;
        self.ident_count += 1;
        self.param_buffer.push_instruction(instruction!(
            DP::Loc,
            [(Ident, param_id)]
        ));
        self.param_buffer.push_parameter(Parameter::from_value(x));
        self.param_buffer.push_parameter(Parameter::from_value(y));
        self.param_buffer.push_parameter(Parameter::from_value(z));
        self.param_buffer.push_tag(Tag::new_value(Pitch, pitch));
        self.param_buffer.push_tag(Tag::new_value(Yaw, yaw));
        self.idents_number_hash.insert(key.to_owned(), param_id);
        param_id
    }

    pub fn use_string(&mut self, name: &str) -> u32 {
        if let Some(id) = self.idents_string_hash.get(name) {
            return *id;
        }
        let param_id = self.ident_count;
        self.ident_count += 1;
        self.param_buffer.push_instruction(instruction!(
            DP::Str,
            [(Ident, param_id), (String, name)]
        ));
        self.idents_string_hash.insert(name.to_owned(), param_id);
        param_id
    }

    pub fn use_variable(&mut self, name: &str, scope: (u8, u16, u8, u16)) -> u32 {
        if let Some(id) = self.idents_variable_hash.get(name) {
            return *id;
        }
        let param_id = self.ident_count;
        self.ident_count += 1;
        self.param_buffer.push_instruction(instruction!(
            DP::Var,
            [(Ident, param_id), (String, name)], [
                Tag::new(scope)
            ]
        ));
        self.idents_variable_hash.insert(name.to_owned(), param_id);
        param_id
    }
    /// Returns a (u32, u32), the first u32 being the ID of the parameter, the second being of its line variable equivalent.
    pub fn use_param(&mut self, name: &str) -> (u32, u32) {
        let var_id = self.use_variable(name, DP::Var::Scope::Line);
        if let Some(id) = self.idents_param_hash.get(name) {
            return (*id, var_id);
        }
        let param_id = self.ident_count;
        self.ident_count += 1;
        self.param_buffer.push_instruction(instruction!(
            DP::Param,
            [(Ident, param_id), (String, name)]
        ));
        self.idents_param_hash.insert(name.to_owned(), param_id);
        (param_id, var_id)
    }
    
    /// Returns a (u32, u32), the first u32 being the ID of the parameter, the second being of its line variable equivalent.
    pub fn use_return_param(&mut self, name: &str) -> (u32, u32) {
        let var_id = self.use_variable(name, DP::Var::Scope::Line);
        if let Some(id) = self.idents_return_param_hash.get(name) {
            return (*id, var_id);
        }
        let param_id = self.ident_count;
        self.ident_count += 1;
        self.param_buffer.push_instruction(instruction!(
            DP::Param,
            [(Ident, param_id), (String, name)], {
                Type: Var
            }
        ));
        self.idents_return_param_hash.insert(name.to_owned(), param_id);
        (param_id, var_id)
    }


    pub fn use_function(&mut self, name: &str) -> u32 {
        if let Some(id) = self.idents_function_hash.get(name) {
            return *id;
        }
        let func_id = self.ident_count;
        // dbg!(name, func_id, self.ident_count);
        self.ident_count += 1;
        self.func_buffer.push_instruction(instruction!(
            DF,
            [(Ident, func_id), (String, name)]
        ));
        self.idents_function_hash.insert(name.to_owned(), func_id);
        func_id
    }


    pub fn use_line_register(&mut self, index: usize) -> u32 {
        if let Some(id) = self.line_register_idents.get(&index) {
            return *id;
        }
        let register_id = self.use_variable(&format!("_xl{}", index), DP::Var::Scope::Line);
        self.line_register_idents.insert(index, register_id);
        self.line_register_indices.insert(register_id, index);
        register_id
    }

    fn bitset_allocate(vec: &mut Vec<u64>) -> usize {
        let mut i = 0;
        loop {
            if i >= vec.len() {
                break;
            }
            let trailing_ones = vec[i].trailing_ones();
            if trailing_ones == 64 {
                i += 1;
                continue;
            } else {
                vec[i] |= 1u64 << trailing_ones;
                return (i*64) + (trailing_ones as usize);
            }
        }
        vec.push(1);
        (vec.len()-1) << 6
    } 

    fn bitset_deallocate(vec: &mut Vec<u64>, ind: usize) {
        let ind_u64 = ind as u64;
        vec[ind >> 6] &= !(1u64 << (ind_u64 & 0b111111));
    } 

    pub fn allocate_line_register(&mut self) -> u32 {
        let index = Self::bitset_allocate(&mut self.allocated_line_registers);
        self.use_line_register(index)
    }

    pub fn allocate_line_register_group(&mut self) -> u64 {
        self.line_register_groups += 1;
        self.line_register_groups - 1
    }

    pub fn allocate_grouped_line_register(&mut self, group: u64) -> u32 {
        let register_ident = self.allocate_line_register();
        if let Some(group) = self.allocated_line_register_groups.get_mut(&self.line_register_groups) {
            group.push(register_ident);
        } else {
            self.allocated_line_register_groups.insert(group.to_owned(), vec![register_ident]);
        }
        register_ident
    }

    pub fn free_line_register(&mut self, register_ident: u32) -> Result<(), CodegenError> {
        let Some(ind) = self.line_register_indices.get(&register_ident) else {
            return CodegenError::err_headless(ErrorRepr::RegisterDeallocationError);
        };
        Self::bitset_deallocate(&mut self.allocated_line_registers, *ind);
        Ok(())
    }

    pub fn free_line_registers(&mut self, register_idents: Vec<u32>) -> Result<(), CodegenError> {
        for register_ident in register_idents {
            self.free_line_register(register_ident)?;
        }
        Ok(())
    }

    pub fn free_line_register_group(&mut self, group: u64) {
        if let Some(group_get) = self.allocated_line_register_groups.get(&group) {
            self.free_line_registers(group_get.clone()).expect("Line register group should free.");
            self.allocated_line_register_groups.remove(&group);
        }
    }


    pub fn constant_void(&mut self) -> u32 {
        // dbg!("Called constant void, current: ", self.constants.void_variable);
        // dbg!(self.idents_variable_hash.get("_c_void"));
        // dbg!(self.ident_count);
        match self.constants.void_variable {
            Some(value) => value,
            None => {
                let value = self.use_variable("_c_void", Scope::Global);
                self.constants.void_variable = Some(value);
                value
            }
        }
    }
}