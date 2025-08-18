use std::{path::PathBuf, sync::LazyLock};

use anyhow::{anyhow, Result};

use crate::{logging::message_box, DLL_PATH};

pub const CONFIG: LazyLock<ColdLoaderConfig> = LazyLock::new(|| {
    let config = read_config().unwrap_or_else(|e| {
        message_box(&format!(
            "Failed to read coldloader.ini: {}",
            e.to_string()
        ));
        std::process::exit(1);
    });
    config
});

#[derive(Debug)]
pub struct ColdLoaderConfig {
    pub app_id: u32,
    pub steamclient64_path: PathBuf
}

pub fn read_config() -> Result<ColdLoaderConfig> {
    let ini_path = DLL_PATH.get().unwrap().join("coldloader.ini");
    let ini = ini::Ini::load_from_file(ini_path).ok();
    let dll_path = DLL_PATH.get().unwrap();

    let steamclient64_path = ini
        .as_ref()
        .and_then(|ini| ini.section(Some("settings")))
        .and_then(|s| s.get("steamclient64"))
        .or(Some("steamclient64.dll"))
        .and_then(|s| {
            let path = dll_path.join(s);
            if path.exists() {
                Some(path)
            } else {
                None
            }
        })
        .ok_or(anyhow!("steamclient64 not found"))?;

    let app_id = ini
        .as_ref()
        .and_then(|ini| ini.section(Some("settings")))
        .and_then(|s| s.get("appid"))
        .and_then(|s| s.parse::<u32>().ok())
        .or_else(|| {
            // read the app_id from steam_settings/steam_appid.txt
            let appid_path = steamclient64_path
                .parent()?
                .join("steam_settings")
                .join("steam_appid.txt");

            let appid_content = std::fs::read_to_string(appid_path).ok()?;

            appid_content.trim().parse::<u32>().ok()
        })
        .ok_or_else(|| anyhow!("appid not found in coldloader.ini or steam_appid.txt"))?;

    let config = ColdLoaderConfig {
        app_id,
        steamclient64_path
    };

    Ok(config)
}
