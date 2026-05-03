use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Param {
    pub enabled: bool,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    pub user_data_dir: Param,
    pub fingerprint: Param,
    pub fingerprint_platform: Param,
    pub fingerprint_platform_version: Param,
    pub fingerprint_brand: Param,
    pub fingerprint_brand_version: Param,
    pub fingerprint_hardware_concurrency: Param,
    pub disable_non_proxied_udp: Param,
    pub lang: Param,
    pub accept_lang: Param,
    pub timezone: Param,
    pub proxy_server: Param,
    pub disable_spoofing: Param,
    pub enable_cdp: bool,
    pub close_after_launch: bool,
    pub auto_seed: bool,
}

impl Config {
    fn config_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("fpui.json")))
            .unwrap_or_else(|| PathBuf::from("fpui.json"))
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&data) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}
