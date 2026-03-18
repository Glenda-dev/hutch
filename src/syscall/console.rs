use crate::kernel::KernelState;
use glenda::cap::ipcmethod;

impl KernelState {
    pub fn handle_console_invocation(
        &self,
        method: usize,
        tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        use glenda::ipc::MsgTag;
        let _msg_tag = MsgTag(tag);
        // Console typically handles generic IPC SEND for print
        if method == ipcmethod::SEND || method == ipcmethod::CALL {
            // Placeholder: Print from MRs
            for &val in &mrs {
                if val == 0 {
                    break;
                }
                print!("{}", val as u8 as char);
            }
        }
        (0, vec![])
    }
}
