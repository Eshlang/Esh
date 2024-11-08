use dfbin::DFBin;

pub struct CodeGenBuffer {
    pub code_buffer: DFBin,
    pub func_buffer: DFBin,
    pub param_buffer: DFBin,
}

impl CodeGenBuffer {
    pub fn new() -> CodeGenBuffer {
        Self {
            code_buffer: DFBin::new(),
            func_buffer: DFBin::new(),
            param_buffer: DFBin::new(),
        }
    }
    pub fn clear(&mut self) {
        self.code_buffer.clear();
        self.func_buffer.clear();
        self.param_buffer.clear();
    }
    pub fn flush(&mut self) -> DFBin {
        let mut flushed = DFBin::new();
        flushed.append_bin_mut(&mut self.func_buffer);
        flushed.append_bin_mut(&mut self.param_buffer);
        flushed.append_bin_mut(&mut self.code_buffer);
        flushed
    }
}