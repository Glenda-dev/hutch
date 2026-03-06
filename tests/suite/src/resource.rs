extern crate glenda;
use glenda::cap::MONITOR_CAP;
use glenda::client::ResourceClient;
use glenda::interface::ResourceService;
use glenda::protocol;
use glenda::sys::sbrk;

pub fn main() {
    glenda::arch::hosted::runtime::crt0_init();
    println!("[TEST-RESOURCE] Starting...");

    // 1. Test SBRK
    let old_brk = sbrk(0).expect("sbrk(0) failed");
    println!("[TEST-RESOURCE] Initial BRK: 0x{:x}", old_brk);

    let new_brk = sbrk(4096).expect("sbrk(4096) failed");
    assert_eq!(new_brk, old_brk);

    let current_brk = sbrk(0).expect("sbrk(0) second call failed");
    assert_eq!(current_brk, old_brk + 4096);
    println!("[TEST-RESOURCE] New BRK: 0x{:x}", current_brk);

    // 2. Test GetCap (Generic Resource)
    let mut client = ResourceClient::new(MONITOR_CAP);
    let fs_cap = client
        .get_cap(
            glenda::ipc::Badge::null(),
            glenda::protocol::resource::ResourceType::from(protocol::resource::FS_ENDPOINT),
            0,
            glenda::cap::CapPtr::null(),
        )
        .expect("Failed to get FS cap");
    println!("[TEST-RESOURCE] Received FS Cap: {:?}", fs_cap);

    println!("[TEST-RESOURCE] PASSED");
}
