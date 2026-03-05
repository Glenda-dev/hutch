use crate::kernel::KernelState;
use glenda::cap::ipcmethod;
use glenda::ipc::MsgTag;

impl KernelState {
    pub fn handle_console_invocation(
        &self,
        method: usize,
        tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        let _msg_tag = MsgTag(tag);
        if method == ipcmethod::SEND || method == ipcmethod::CALL {
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
