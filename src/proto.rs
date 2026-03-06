use crate::config::Config;
use bincode;
use glenda::protocol::hosted::{HostedMessage, HostedReply};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub fn handle_client(
    mut stream: UnixStream,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let kernel = crate::kernel::KernelState::new(config);

    loop {
        // 先读取 4 字节长度
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).is_err() {
            break; // EOF
        }
        let len = u32::from_le_bytes(len_buf) as usize;

        // 读取消息体
        let mut msg_buf = vec![0u8; len];
        stream.read_exact(&mut msg_buf)?;

        let msg: HostedMessage = bincode::deserialize(&msg_buf)?;

        match msg {
            HostedMessage::SysInvoke { cptr, method, utcb_ptr } => {
                let ret = kernel.invoke_cap(cptr, method, utcb_ptr);
                let reply = HostedReply::Success { ret };
                
                let bytes = bincode::serialize(&reply)?;
                let reply_len = bytes.len() as u32;
                
                // 写入长度和消息体
                stream.write_all(&reply_len.to_le_bytes())?;
                stream.write_all(&bytes)?;
                stream.flush()?;
            }
        }
    }
    Ok(())
}
