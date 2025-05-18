use decompiler::Decompiler;
use dfbin::{Constants::{Actions::Seg, Parents}, DFBin};

use crate::{codeline::Codeline, errors::OptimizerError};
use dfbin::instruction;

#[derive(Clone, Debug, PartialEq)]
pub struct Buffer {
    pub func_buffer: DFBin,
    pub param_buffer: DFBin,
    pub code_lines: Vec<DFBin>,
    pub code_branches: Vec<Codeline>,
}

#[allow(non_snake_case, non_upper_case_globals)]
impl Buffer {
    pub fn new(bin: DFBin) -> Result<Self, OptimizerError> {
        let mut make = Self {
            func_buffer: DFBin::new(),
            param_buffer: DFBin::new(),
            code_lines: Vec::new(),
            code_branches: Vec::new(),
        };
        make.append_bin(bin)?;
        make.get_code_branches()?;
        return Ok(make);
    }

    pub fn append_bin(&mut self, bin: DFBin) -> Result<(), OptimizerError> {
        let mut current_segment = (0, 0);
        for instruction in bin.instructions() {
            if matches!(instruction.action, Seg::Code | Seg::Func | Seg::Param) {
                current_segment = instruction.action;
                continue;
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
        for (codeline_ind, line) in self.code_lines.iter().enumerate() {
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

        final_buffer
    }


}