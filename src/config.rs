use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub hutch: HutchConfig,
    pub sandbox: SandboxConfig,
    pub resources: ResourceConfig,
    #[serde(default)]
    pub vfio: VfioConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HutchConfig {
    pub socket_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SandboxConfig {
    pub root_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResourceConfig {
    pub heap_start: usize,
    pub heap_size: usize,
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        toml::from_str(&content).expect("Failed to parse config file")
    }

    pub fn default() -> Self {
        Self {
            hutch: HutchConfig { socket_path: "/tmp/glenda_hutch.sock".to_string() },
            sandbox: SandboxConfig { root_path: "/tmp/glenda_root".to_string() },
            resources: ResourceConfig { heap_start: 0x10000000, heap_size: 0x10000000 },
            vfio: VfioConfig::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct VfioConfig {
    pub devices: Vec<VfioDeviceConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VfioDeviceConfig {
    pub name: String,
    pub compatible: Vec<String>,
    pub group_id: u32,
    pub vfio_name: String,
}
