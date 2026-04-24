use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut config_path = None;
    let mut listen_path = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    config_path = Some(args[i + 1].as_str());
                    i += 1;
                }
            }
            "--listen" | "-l" | "--socket" => {
                if i + 1 < args.len() {
                    listen_path = Some(args[i + 1].to_string());
                    i += 1;
                }
            }
            _ => {
                if config_path.is_none() && !args[i].starts_with('-') {
                    config_path = Some(args[i].as_str());
                }
            }
        }
        i += 1;
    }

    if let Err(e) = start_server(config_path, listen_path) {
        eprintln!("Hutch failed: {}", e);
        std::process::exit(1);
    }
}
pub mod config;
pub mod io;
pub mod kernel;
pub mod proto;
pub mod service;
pub mod syscall;
pub mod utils;

use crate::config::Config;
use std::fs;

use std::os::unix::net::UnixListener;
use std::thread;

pub fn start_server(config_path: Option<&str>, listen_path: Option<String>) -> std::io::Result<()> {
    crate::kernel::init::init();
    let mut config = match config_path {
        Some(p) => Config::from_file(p),
        None => Config::default(),
    };

    if let Some(path) = listen_path {
        config.hutch.socket_path = path;
    }

    let path = &config.hutch.socket_path;
    if fs::metadata(path).is_ok() {
        fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;
    println!("[hutch] Listening on {}", path);

    let kernel_state = crate::kernel::KernelState::new(config.clone());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let ks = kernel_state.clone();
                thread::spawn(move || {
                    if let Err(e) = crate::proto::handle_client(stream, ks) {
                        eprintln!("[hutch] Client error: {}", e);
                    }
                });
            }
            Err(err) => {
                eprintln!("[hutch] Accept error: {}", err);
            }
        }
    }
    Ok(())
}
