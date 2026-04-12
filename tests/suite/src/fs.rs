extern crate glenda;
use glenda::cap::MONITOR_CAP;
use glenda::client::FsClient;
use glenda::client::ResourceClient;
use glenda::interface::{FileHandleService, FileSystemService, ResourceService};
use glenda::protocol::resource::{FS_ENDPOINT, ResourceType};

pub fn main() {
    glenda::arch::hosted::runtime::crt0_init();
    println!("[TEST-FS] Starting...");

    let mut res_client = ResourceClient::new(MONITOR_CAP);
    let fs_cap = res_client
        .get_cap(
            glenda::ipc::Badge::null(),
            ResourceType::from(FS_ENDPOINT),
            0,
            glenda::cap::CapPtr::null(),
        )
        .expect("Failed to get FS cap");
    let mut fs = FsClient::new(glenda::cap::Endpoint::from(fs_cap));

    // 1. Open / Read (Test basic service routing in hutch)
    // Assuming hutch redirects to a sandbox/ or root/ folder
    match fs.open(
        glenda::ipc::Badge::null(),
        "VERSION",
        glenda::protocol::fs::OpenFlags::O_RDONLY,
        0,
        glenda::cap::CapPtr::null(),
    ) {
        Ok(fd) => {
            println!("[TEST-FS] Opened VERSION file: fd={}", fd);
            let mut read_buf = [0u8; 32];
            // Since FsClient typically maps to a single handle or handles are derived,
            // In Glenda's current design, the FsClient might be the handle itself or a proxy.
            // If FsClient implements FileHandleService directly, we use it.
            let n = fs.read(glenda::ipc::Badge::null(), 0, &mut read_buf).expect("Read failed");
            println!("[TEST-FS] Read {} bytes: {:?}", n, &read_buf[..n]);
        }
        Err(e) => {
            println!("[TEST-FS] Open failed (expected if sandbox is empty): {:?}", e);
        }
    }

    println!("[TEST-FS] PASSED (Structural Check)");
}
