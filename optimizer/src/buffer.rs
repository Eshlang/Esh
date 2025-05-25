use std::collections::HashMap;

use dfbin::{enums::{Instruction, Parameter, ParameterValue}, Constants::{Actions::Seg, Parents}, DFBin};

use crate::{codeline::Codeline, errors::{ErrorRepr, OptimizerError}};
use dfbin::instruction;


#[derive(Clone, Debug, PartialEq)]
pub struct Buffer {
    pub func_buffer: DFBin,
    pub param_buffer: DFBin,
    pub code_lines: Vec<DFBin>,
    pub code_branches: Vec<Codeline>,
    pub ident_count: u32,
    pub ident_definitions: HashMap<u32, (Instruction, Vec<ParameterValue>)>,
}

#[allow(non_snake_case, non_upper_case_globals)]
impl Buffer {
    pub fn new(bin: DFBin) -> Result<Self, OptimizerError> {
        let mut make = Self {
            func_buffer: DFBin::new(),
            param_buffer: DFBin::new(),
            code_lines: Vec::new(),
            code_branches: Vec::new(),
            ident_count: 0,
            ident_definitions: HashMap::new(),
        };
        make.append_bin(bin)?;
        make.get_code_branches()?;
        return Ok(make);
    }

    /// Put a DF/DP instruction but don't include the first ident that would be set, the buffer will decide an ident and return it.
    /// Please try to use this instead of ``self.func/param_buffer.push_instruction()``!!!
    pub fn add_definition(&mut self, mut instruction: Instruction) -> u32 {
        if !matches!(instruction.action.0, Parents::DP | Parents::DF) {
            panic!("uhm i said only put a df/dp instruction in here");
        }
        let ident = self.ident_count;
        self.ident_count += 1;
        let param_values = instruction.params.iter().map(|x| x.value.clone()).collect();
        instruction.params.insert(0, Parameter::from_ident(ident));
        self.ident_definitions.insert(ident, (instruction.clone(), param_values));
        match instruction.action.0 {
            Parents::DP => {
                self.param_buffer.push_instruction(instruction);
            }
            Parents::DF => {
                self.func_buffer.push_instruction(instruction);
            }
            _ => { panic!("uhm i said only put a df/dp instruction in here"); }
        }
        ident
    }

    /// Loads all the DFBIN related things into the current buffer from the reference buffer
    pub fn load_state(&mut self, ref_buffer: Buffer) {
        self.func_buffer = ref_buffer.func_buffer;
        self.param_buffer = ref_buffer.param_buffer;
        self.ident_count = ref_buffer.ident_count;
        self.ident_definitions = ref_buffer.ident_definitions;
    }

    /// Returns the ident that would be returned by ``.add_definition()``, and *doesn't* free it (you still have to use add definition to free it)
    pub fn next_ident(&self) -> u32 {
        self.ident_count
    }

    pub fn append_bin(&mut self, bin: DFBin) -> Result<(), OptimizerError> {
        let mut current_segment = (0, 0);
        for instruction in bin.instructions() {
            if matches!(instruction.action, Seg::Code | Seg::Func | Seg::Param) {
                current_segment = instruction.action;
                continue;
            }
            if matches!(instruction.action.0, Parents::DP | Parents::DF) {
                let ident = instruction.params[0].value.as_ident().map_err(|_| OptimizerError::new_headless(ErrorRepr::ExpectedIdentifier))?;
                let mut param_values: Vec<ParameterValue> = instruction.params.iter().map(|x| x.value.clone()).collect();
                param_values.remove(0);
                self.ident_definitions.insert(ident, (instruction.clone(), param_values));
                if self.ident_count < ident + 1 {
                    self.ident_count = ident + 1;
                }
            }
            match current_segment {
                Seg::Func => {
                    self.func_buffer.push_instruction(instruction);
                }
                Seg::Param => {
                    self.param_buffer.push_instruction(instruction);
                }
                Seg::Code => {
                    if matches!(instruction.action.0, Parents::Func | Parents::FuncA | Parents::Plev | Parents::Enev) { // line starters
                        self.code_lines.push(DFBin::new())
                    }
                    let len = self.code_lines.len();
                    if len > 0 { //ignore any dead pre-line-starter code
                        self.code_lines[len - 1].push_instruction(instruction);
                    }
                }
                _ => {} //dead instructions placed before any segment
            }
        }
        Ok(())
    }

    pub fn get_code_branches(&mut self) -> Result<(), OptimizerError> {
        for (_codeline_ind, line) in self.code_lines.iter().enumerate() {
            // println!("\n\n\n(ASM) Codeline #{}\n------------------------------------------------\n{}\n------------------------------------------------", 
            //     codeline_ind,
            //     Decompiler::new(line.clone()).expect("Should decompile").decompile().expect("Should decompile"));
            let codeline = Codeline::from_bin(line.clone())?;
            
            // println!("\n\n\n(Branches) Codeline #{}\n------------------------------------------------\n{}\n------------------------------------------------", 
            //     codeline_ind,
            //     Decompiler::new(codeline.clone().to_bin()).expect("Should decompile").decompile().expect("Should decompile"));
            // for (codeline_branch_ind, codeline_branch) in codeline.clone().branch_list.iter().enumerate() {
            //     // println!("\n\n\n(Branches) Codeline #{}, Branch #{}, ({:#?})\n------------------------------------------------\n{}\n------------------------------------------------", 
            //     //     codeline_ind, codeline_branch_ind, codeline_branch,
            //     //     Decompiler::new(codeline_branch.clone().instructions(&codeline.branch_list)).expect("Should decompile").decompile().expect("Should decompile"));
            // }
            self.code_branches.push(codeline)
        }
        
        // println!("Branches:\n------------------------------------------------\n\n{:#?}\n\n------------------------------------------------", make.branches);

        Ok(())
    }

    pub fn flush(&mut self) -> DFBin {
        let mut final_buffer = DFBin::new();
        final_buffer.push_instruction(instruction!(Seg::Func));
        final_buffer.append_bin_mut(&mut self.func_buffer);
        final_buffer.push_instruction(instruction!(Seg::Param));
        final_buffer.append_bin_mut(&mut self.param_buffer);
        final_buffer.push_instruction(instruction!(Seg::Code));
        for branch in &self.code_branches {
            final_buffer.append_bin(&branch.clone().to_bin());
        }
        self.ident_count = 0;

        final_buffer
    }


}