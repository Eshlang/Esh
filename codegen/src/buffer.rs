use std::collections::HashMap;
use dfbin::{instruction, Constants, DFBin};
use Constants::Tags::DP;

pub struct CodeGenBuffer {
    pub code_buffer: DFBin,
    pub func_buffer: DFBin,
    pub param_buffer: DFBin,
    ident_count: u32,
    variable_hash: HashMap<String, u32>,
    param_hash: HashMap<String, u32>,
    function_hash: HashMap<String, u32>,
}

impl CodeGenBuffer {
    pub fn new() -> CodeGenBuffer {
        Self {
            code_buffer: DFBin::new(),
            func_buffer: DFBin::new(),
            param_buffer: DFBin::new(),
            ident_count: 0,
            variable_hash: HashMap::new(),
            param_hash: HashMap::new(),
            function_hash: HashMap::new(),
        }
    }
    fn clear_variables(&mut self) {
        self.ident_count = 0;
        self.variable_hash.clear();
        self.param_hash.clear();
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
    pub fn use_variable(&mut self, name: &str, scope: (u8, u16, u8, u16)) -> u32 {
        if let Some(id) = self.variable_hash.get(name) {
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
        self.variable_hash.insert(name.to_owned(), param_id);
        param_id
    }
    /// Returns a (u32, u32), the first u32 being the ID of the parameter, the second being of its line variable equivalent.
    pub fn use_param(&mut self, name: &str) -> (u32, u32) {
        let var_id = self.use_variable(name, DP::Var::Scope::Line);
        if let Some(id) = self.param_hash.get(name) {
            return (*id, var_id);
        }
        let param_id = self.ident_count;
        self.ident_count += 1;
        self.param_buffer.push_instruction(instruction!(
            DP::Param,
            [(Ident, param_id), (String, name)]
        ));
        self.param_hash.insert(name.to_owned(), param_id);
        (param_id, var_id)
    }


    pub fn use_function(&mut self, name: &str) -> u32 {
        if let Some(id) = self.function_hash.get(name) {
            return *id;
        }
        let func_id = self.ident_count;
        self.ident_count += 1;
        self.func_buffer.push_instruction(instruction!(
            DF,
            [(Ident, func_id), (String, name)]
        ));
        self.function_hash.insert(name.to_owned(), func_id);
        func_id
    }
}