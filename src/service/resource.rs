pub struct ResourceManager {
    pub heap_start: usize,
    pub heap_size: usize,
    pub brk: usize,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            heap_start: 0x10000000,
            heap_size: 0x10000000, // 256MB
            brk: 0x10000000,
        }
    }

    pub fn sbrk(&mut self, incr: isize) -> usize {
        let old_brk = self.brk;
        self.brk = (self.brk as isize + incr) as usize;
        old_brk
    }
}
