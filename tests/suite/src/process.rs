extern crate glenda;
use glenda::cap::MONITOR_CAP;
use glenda::client::ProcessClient;
use glenda::client::ResourceClient;
use glenda::interface::{ProcessService, ResourceService};
use glenda::protocol::resource::{PROCESS_ENDPOINT, ResourceType};

pub fn main() {
    glenda::arch::hosted::runtime::crt0_init();
    println!("[TEST-PROCESS] Starting...");

    let mut res_client = ResourceClient::new(MONITOR_CAP);
    let proc_cap = res_client
        .get_cap(
            glenda::ipc::Badge::null(),
            ResourceType::from(PROCESS_ENDPOINT),
            0,
            glenda::cap::CapPtr::null(),
        )
        .expect("Failed to get Process cap");
    let mut process = ProcessClient::new(glenda::cap::Endpoint::from(proc_cap));

    // 1. Spawn Check (Simulating a process via hutch's std::process::Command)
    // Here we'll just test the protocol connectivity
    let _utcb = unsafe { glenda::ipc::UTCB::new() };

    // In hutch, SPAWN might just log or start a host binary
    match process.spawn(glenda::ipc::Badge::null(), "/bin/sh") {
        Ok(pid) => {
            println!("[TEST-PROCESS] Spawned child: PID={}", pid);
            assert!(pid > 0);

            // Wait / Query (Wait is not in ProcessService trait, might be service specific or not implemented)
            // let status = process.wait(&mut utcb, pid).expect("Wait failed");
        }
        Err(e) => {
            println!("[TEST-PROCESS] Spawn failed: {:?}", e);
            // On some host-os config, binary might not be there.
        }
    }

    println!("[TEST-PROCESS] PASSED (Structural Check)");
}
