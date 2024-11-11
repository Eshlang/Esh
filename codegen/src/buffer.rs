use std::collections::HashMap;
use dfbin::{enums::{Parameter, ParameterValue}, instruction, Constants, DFBin};
use Constants::Tags::DP;

pub struct CodeGenBuffer {
    pub code_buffer: DFBin,
    pub func_buffer: DFBin,
    pub param_buffer: DFBin,
    ident_count: u32,
    idents_number_hash: HashMap<String, u32>,
    idents_string_hash: HashMap<String, u32>,
    idents_variable_hash: HashMap<String, u32>,
    idents_param_hash: HashMap<String, u32>,
    idents_function_hash: HashMap<String, u32>,
    line_register_idents: HashMap<usize, u32>,
}

impl CodeGenBuffer {
    pub fn new() -> CodeGenBuffer {
        Self {
            code_buffer: DFBin::new(),
            func_buffer: DFBin::new(),
            param_buffer: DFBin::new(),
            ident_count: 0,
            idents_number_hash: HashMap::new(),
            idents_string_hash: HashMap::new(),
            idents_variable_hash: HashMap::new(),
            idents_param_hash: HashMap::new(),
            idents_function_hash: HashMap::new(),
            line_register_idents: HashMap::new(),
        }
    }
    fn clear_variables(&mut self) {
        self.ident_count = 0;
        self.idents_variable_hash.clear();
        self.idents_param_hash.clear();
        self.line_register_idents.clear();
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

    fn get_key(params: Vec<ParameterValue>) -> String {
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


    pub fn use_function(&mut self, name: &str) -> u32 {
        if let Some(id) = self.idents_function_hash.get(name) {
            return *id;
        }
        let func_id = self.ident_count;
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
        register_id
    }
}