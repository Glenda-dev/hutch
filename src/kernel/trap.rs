use crate::kernel::KernelState;
use glenda::cap::ipcmethod;
use glenda::protocol::kernel::*;

impl KernelState {
    // Trap module for sub-routine exceptions handling
    pub fn handle_exception(
        &self,
        fault_ep: usize,
        cause: usize,
        pc: usize,
        value: usize,
    ) -> (usize, Vec<usize>) {
        println!("[hutch] Exception trapped: cause={:#x}, pc={:#x}, tval={:#x}", cause, pc, value);

        let tag = match cause {
            12 | 13 | 15 => PAGE_FAULT, // Instruction, Load, Store page fault
            2 => ILLEGAL_INSTRUCTION,
            3 => BREAKPOINT,
            5 | 7 => ACCESS_FAULT,
            4 | 6 => ACCESS_MISALIGNED,
            _ => UNKNOWN_FAULT,
        };

        let mrs = match tag {
            PAGE_FAULT => vec![value, pc, cause],
            ILLEGAL_INSTRUCTION => vec![value, pc],
            BREAKPOINT => vec![pc],
            ACCESS_FAULT => vec![value, pc],
            ACCESS_MISALIGNED => vec![value, pc],
            _ => vec![cause, value, pc],
        };

        // Exception processing generally does a CALL to get resumed
        self.handle_endpoint_invocation(fault_ep, ipcmethod::CALL, tag, mrs)
    }
}
