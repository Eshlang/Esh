use dfbin::{Constants::{Actions::Seg, Parents}, DFBin};

use crate::errors::OptimizerError;

#[derive(Clone, Debug, PartialEq)]
pub struct Buffer {
    pub func_buffer: DFBin,
    pub param_buffer: DFBin,
    pub code_lines: Vec<DFBin>
}

#[allow(non_snake_case, non_upper_case_globals)]
impl Buffer {
    pub fn new(bin: DFBin) -> Result<Self, OptimizerError> {
        let mut make = Self {
            func_buffer: DFBin::new(),
            param_buffer: DFBin::new(),
            code_lines: Vec::new(),
        };
        make.append_bin(bin)?;
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
}