extern crate glenda;
use glenda::cap::CapPtr;
use glenda::cap::Endpoint;
use glenda::ipc::{Badge, UTCB};
use std::thread;
use std::time::Duration;

pub fn main() {
    glenda::arch::hosted::runtime::crt0_init();
    println!("[TEST-IPC] IPC & Notify Test Starting...");

    let ep = Endpoint::from(CapPtr::from(8)); // Use generic test endpoint cptr from hutch

    // 1. Simple Sync IPC (SEND/RECV)
    println!("[TEST-IPC] Step 1: Sync SEND/RECV");
    let receiver = thread::spawn(move || {
        let mut utcb = unsafe { UTCB::new() };
        ep.recv(&mut utcb).expect("Recv failed");
        let msg = utcb.get_mr(0);
        println!("[Receiver] Received: {}", msg);
        assert_eq!(msg, 0xcafe);
    });

    thread::sleep(Duration::from_millis(100));
    {
        let mut utcb = unsafe { UTCB::new() };
        utcb.set_mr(0, 0xcafe);
        ep.send(&mut utcb).expect("Send failed");
    }
    receiver.join().unwrap();

    // 2. Notification (Notify/Recv)
    println!("[TEST-IPC] Step 2: Notify Mechanism");
    let notify_receiver = thread::spawn(move || {
        let mut utcb = unsafe { UTCB::new() };
        ep.recv(&mut utcb).expect("Recv Notify failed");
        let badge = utcb.get_badge();
        println!("[Receiver] Received Badge: 0x{:x}", badge.bits());
        assert_eq!(badge.bits(), 0x1337);
    });

    thread::sleep(Duration::from_millis(100));
    ep.notify(Badge::new(0x1337)).expect("Notify failed");
    notify_receiver.join().unwrap();

    println!("[TEST-IPC] PASSED");
}
