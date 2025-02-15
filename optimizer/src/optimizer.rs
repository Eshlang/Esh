use dfbin::DFBin;

pub struct Optimizer {
    pub bin: DFBin
}

impl Optimizer {
    pub fn new(bin: DFBin) -> Self {
        Self {
            bin
        }
    }
}