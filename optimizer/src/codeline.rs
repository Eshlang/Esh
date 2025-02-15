use std::rc::Rc;

use dfbin::{enums::Instruction, instruction, Constants::Parents, DFBin};

use crate::errors::OptimizerError;

#[derive(Clone, Debug, PartialEq)]
pub enum CodelineBranchLog {
    Codeblocks(Vec<Instruction>),
    Branch(CodelineBranch)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodelineBranch {
    pub branch_type: CodelineBranchType,
    pub root: Instruction,
    pub body: Rc<Vec<CodelineBranchLog>>
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodelineBranchType {
    If, Repeat
}

#[derive(Clone, Debug, PartialEq)]
pub struct Codeline {
    pub root_instruction: Instruction,
    pub body_instructions: Vec<Instruction>,
    pub branches: Rc<Vec<CodelineBranchLog>>,

    buffer: DFBin,
    pointer: usize,
    met_else: bool,
}

impl Codeline {
    pub fn from_bin(bin: DFBin) -> Result<Self, OptimizerError> {
        let mut instructions = bin.instructions();
        let bin_instructions = instructions.clone();
        let root_instruction = instructions.remove(0);
        let mut make = Self {
            root_instruction,
            body_instructions: instructions,
            branches: Rc::new(Vec::new()),

            buffer: DFBin::new(),
            pointer: 0,
            met_else: false,
        };
        make.branches = make.evaluate_branch()?;

        Ok(make)
    }

    fn evaluate_branch(&mut self) -> Result<Rc<Vec<CodelineBranchLog>>, OptimizerError> {
        let mut branch_logs = Vec::new();
        let mut instructions = Vec::new();
        loop {
            let Some(instruction) = self.body_instructions.get(self.pointer) else {
                break;
            };
            self.pointer += 1;
            match (instruction.action.0, self.met_else) {
                (Parents::EndIf | Parents::EndRep, false) => {
                    break;
                }
                (Parents::Else, false) => {
                    self.pointer -= 1;
                    self.met_else = true;
                    break;
                }
                (Parents::Varif | Parents::Enif | Parents::Plif | Parents::Gmif | Parents::Rep, false) | (Parents::Else, true) => {
                    self.met_else = false;
                    if instructions.len() > 0 {
                        branch_logs.push(CodelineBranchLog::Codeblocks(instructions.clone()));
                        instructions.clear();
                    }
                    branch_logs.push(CodelineBranchLog::Branch(CodelineBranch {
                        branch_type: match instruction.action.0 {
                            Parents::Rep => CodelineBranchType::Repeat,
                            _ => CodelineBranchType::If
                        },
                        root: instruction.clone(),
                        body: self.evaluate_branch()?
                    }));
                }
                _ => {
                    self.met_else = false;
                    instructions.push(instruction.clone());
                }
            }
        }
        if instructions.len() > 0 {
            branch_logs.push(CodelineBranchLog::Codeblocks(instructions.clone()));
        }
        return Ok(Rc::new(branch_logs));
    }

    pub fn to_bin(&mut self) -> DFBin {
        self.buffer = DFBin::new();
        self.buffer.push_instruction_ref(&self.root_instruction);
        self.add_buffer(self.branches.clone());
        self.buffer.clone()
    }

    fn add_buffer(&mut self, branch_logs: Rc<Vec<CodelineBranchLog>>) {
        for branch_log in branch_logs.as_ref() {
            match branch_log {
                CodelineBranchLog::Branch(branch) => {
                    if self.buffer.read_instruction_at_index(self.buffer.len() - 1).expect("There should be an instruction here.").action.0 == Parents::EndIf && branch.root.action.0 == Parents::Else {
                        self.buffer.remove_at_index(self.buffer.len() - 1).expect("This instruction should be removed.")
                    }
                    self.buffer.push_instruction_ref(&branch.root);
                    self.add_buffer(branch.body.clone());
                    match branch.branch_type {
                        CodelineBranchType::If => { self.buffer.push_instruction(instruction!(EndIf)); },
                        CodelineBranchType::Repeat =>  { self.buffer.push_instruction(instruction!(EndRep)); }
                    };
                }
                CodelineBranchLog::Codeblocks(codeblocks) => {
                    self.buffer.append_instructions(codeblocks.clone());
                }
            }
        }
    }
}