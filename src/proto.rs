use crate::config::Config;
use bincode;
use glenda::protocol::hosted::{HostedMessage, HostedReply};
use std::io::{Write};
use std::os::unix::net::UnixStream;

pub fn handle_client(
    mut stream: UnixStream,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let kernel = crate::kernel::KernelState::new(config);

    loop {
        let msg: HostedMessage = match bincode::deserialize_from(&stream) {
            Ok(m) => m,
            Err(_) => break, // EOF or error
        };

        match msg {
            HostedMessage::SysInvoke { cptr, method, utcb_ptr } => {
                let ret = kernel.invoke_cap(cptr, method, utcb_ptr);
                let reply = HostedReply::Success { ret };
                let bytes = bincode::serialize(&reply)?;
                stream.write_all(&bytes)?;
            }
        }
    }
    Ok(())
}
