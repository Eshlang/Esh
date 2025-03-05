use dfbin::{enums::Instruction, instruction, Constants::Parents, DFBin};

use crate::errors::OptimizerError;

#[derive(Clone, Debug, PartialEq)]
pub enum CodelineBranchLog {
    Codeblocks(Vec<Instruction>),
    Branch(usize)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodelineBranch {
    pub branch_type: CodelineBranchType,
    pub root: Instruction,
    pub body: Vec<CodelineBranchLog>,
    pub depth: usize,
}

impl CodelineBranch {
    pub fn add_instructions(&self, branches: &Vec<CodelineBranch>, mut buffer: &mut DFBin) {
        for branch_log in &self.body {
            match branch_log {
                CodelineBranchLog::Branch(branch_index) => {
                    let branch = &branches[*branch_index];
                    if buffer.read_instruction_at_index(buffer.len() - 1).expect("There should be an instruction here.").action.0 == Parents::EndIf && branch.root.action.0 == Parents::Else {
                        buffer.remove_at_index(buffer.len() - 1).expect("This instruction should be removed.")
                    }
                buffer.push_instruction_ref(&branch.root);
                    branch.add_instructions(&branches, &mut buffer);
                    let branch = &branches[*branch_index];
                    match branch.branch_type {
                        CodelineBranchType::If => { buffer.push_instruction(instruction!(EndIf)); },
                        CodelineBranchType::Repeat =>  { buffer.push_instruction(instruction!(EndRep)); }
                    };
                }
                CodelineBranchLog::Codeblocks(codeblocks) => {
                    buffer.append_instructions(codeblocks.clone());
                }
            }
        }
    }

    pub fn instructions(&self, branches: &Vec<CodelineBranch>) -> DFBin {
        let mut buffer = DFBin::new();
        self.add_instructions(branches, &mut buffer);
        buffer
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum CodelineBranchType {
    If, Repeat
}

#[derive(Clone, Debug, PartialEq)]
pub struct Codeline {
    pub root_instruction: Instruction,
    pub body_instructions: Vec<Instruction>,
    pub root_branch: Vec<CodelineBranchLog>,
    pub branch_list: Vec<CodelineBranch>,
    pub branches_by_depth: Vec<Vec<usize>>,

    buffer: DFBin,
    pointer: usize,
    met_else: bool,
}

impl Codeline {
    pub fn from_bin(bin: DFBin) -> Result<Self, OptimizerError> {
        let mut instructions = bin.instructions();
        let root_instruction = instructions.remove(0);
        let mut make = Self {
            root_instruction,
            body_instructions: instructions,
            root_branch: Vec::new(),
            branch_list: Vec::new(),
            branches_by_depth: Vec::new(),

            buffer: DFBin::new(),
            pointer: 0,
            met_else: false,
        };
        make.root_branch = make.evaluate_branch(1)?;

        Ok(make)
    }

    fn evaluate_branch(&mut self, depth: usize) -> Result<Vec<CodelineBranchLog>, OptimizerError> {
        let mut branch_logs = Vec::new();
        let mut instructions = Vec::new();
        loop {
            let Some(instruction) = self.body_instructions.get(self.pointer) else {
                break;
            };
            let instruction = instruction.clone();
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
                    let evaluated_branch = self.evaluate_branch(depth+1)?;
                    let branch_index = self.branch_list.len();
                    self.branch_list.push(CodelineBranch {
                        branch_type: match instruction.action.0 {
                            Parents::Rep => CodelineBranchType::Repeat,
                            _ => CodelineBranchType::If
                        },
                        root: instruction,
                        body: evaluated_branch,
                        depth
                    });
                    branch_logs.push(CodelineBranchLog::Branch(branch_index));
                    while self.branches_by_depth.get(depth).is_none() {
                        self.branches_by_depth.push(Vec::new());
                    }
                    self.branches_by_depth[depth].push(branch_index);
                }
                _ => {
                    self.met_else = false;
                    instructions.push(instruction);
                }
            }
        }
        if instructions.len() > 0 {
            branch_logs.push(CodelineBranchLog::Codeblocks(instructions.clone()));
        }
        return Ok(branch_logs);
    }

    pub fn to_bin(&mut self) -> DFBin {
        self.buffer = DFBin::new();
        self.buffer.push_instruction_ref(&self.root_instruction);
        self.add_buffer(self.root_branch.clone());
        self.buffer.clone()
    }

    fn add_buffer(&mut self, branch_logs: Vec<CodelineBranchLog>) {
        for branch_log in branch_logs {
            match branch_log {
                CodelineBranchLog::Branch(branch_index) => {
                    let branch = &self.branch_list[branch_index];
                    if self.buffer.read_instruction_at_index(self.buffer.len() - 1).expect("There should be an instruction here.").action.0 == Parents::EndIf && branch.root.action.0 == Parents::Else {
                        self.buffer.remove_at_index(self.buffer.len() - 1).expect("This instruction should be removed.")
                    }
                    self.buffer.push_instruction_ref(&branch.root);
                    self.add_buffer(branch.body.clone());
                    let branch = &self.branch_list[branch_index];
                    match &branch.branch_type {
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