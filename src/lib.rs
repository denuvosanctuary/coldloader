mod logging;
mod ini;

use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::{Path, PathBuf}, sync::{LazyLock, OnceLock}};

use anyhow::{anyhow, Result};
use ini::CONFIG;
use winapi::{shared::minwindef::{BOOL, DWORD, HMODULE, LPVOID, TRUE}, um::{libloaderapi::{GetModuleFileNameW, LoadLibraryW}, winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH}}};

static DLL_PATH: OnceLock<PathBuf> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(module: HMODULE, reason: DWORD, _reserved: LPVOID) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            let dll_path = unsafe {
                let mut buffer = [0u16; 1024];
                let len = GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32);
                let binding = OsString::from_wide(&buffer[..len as usize]).into_string().unwrap();
                let path = Path::new(binding.as_str());
                path.parent().unwrap().to_path_buf()
            };
            DLL_PATH.set(dll_path).unwrap();

            logging::init_logger();
            logging::setup_panic_handler();

            initialize().unwrap_or_else(|e| {
                log::error!("Failed to initialize: {}", e);
                std::process::exit(1);
            });

            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(CONFIG.cleanup_delay));
                log::info!("Performing cleanup in background thread");
                cleanup();
            });
        }
        DLL_PROCESS_DETACH => {
            log::info!("DLL_PROCESS_DETACH called, performing cleanup");
            cleanup();
        }
        _ => {}
    }
    
    TRUE
}

static STEAM_UNIVERSE: &str = "public";
static PROCESS_ID: LazyLock<u32> = LazyLock::new(|| unsafe {
    winapi::um::processthreadsapi::GetCurrentProcessId()
});

fn patch_registry() -> Result<()> {
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags("Software\\Valve\\Steam\\ActiveProcess", winreg::enums::KEY_ALL_ACCESS)
        .map(|key| {
            key.set_value("pid", &*PROCESS_ID).unwrap();
            key.set_value("SteamClientDll64", &CONFIG.steamclient64_path.to_string_lossy().to_string()).unwrap();
            key.set_value("Universe", &STEAM_UNIVERSE).unwrap();
        })
        .map_err(|e| {
            log::error!("Unable to patch Registry (HKCU), error = {:?}", e);
            e
        })?;

    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags("Software\\Valve\\Steam", winreg::enums::KEY_ALL_ACCESS)
        .map(|key| {
            let client_path = CONFIG.steamclient64_path.clone();
            key.set_value("SteamPath", &client_path.parent().unwrap().to_str().unwrap()).unwrap();
            key.set_value("RunningAppID", &CONFIG.app_id).unwrap();
        })
        .map_err(|e| {
            log::error!("Unable to patch Registry (HKCU #2), error = {:?}", e);
            e
        })?;

    Ok(())
}

fn set_steam_env_vars(app_id: u32) {
    unsafe {
        std::env::set_var("SteamAppId", app_id.to_string());
        std::env::set_var("SteamGameId", app_id.to_string());
    
        std::env::set_var("SteamClientLaunch", "1");
        std::env::set_var("SteamEnv", "1");
    }

    log::info!("Set Steam environment variables");
}

fn initialize() -> Result<()> {
    set_steam_env_vars(CONFIG.app_id);

    // Patch the registry
    patch_registry()?;
    
    // Load the DLLs
    let client_64_path = CONFIG.steamclient64_path.to_string_lossy().to_string();
    let client_64_dll = unsafe { LoadLibraryW(format!("{}\0", client_64_path).encode_utf16().collect::<Vec<u16>>().as_ptr()) };
    if client_64_dll.is_null() {
        return Err(anyhow!("Failed to load steamclient64.dll"));
    }

    log::info!("Loaded steamclient64.dll");

    let gameoverlay_64_path = std::env::current_exe().unwrap().parent().unwrap().join("gameoverlayrenderer64.dll");
    if gameoverlay_64_path.exists() {
        let gameoverlay_64_dll = unsafe { LoadLibraryW(format!("{}\0", gameoverlay_64_path.to_string_lossy()).encode_utf16().collect::<Vec<u16>>().as_ptr()) };
        if gameoverlay_64_dll.is_null() {
            return Err(anyhow!("Failed to load gameoverlayrenderer64.dll"));
        }

        log::info!("Loaded gameoverlayrenderer64.dll");
    } else {
        log::warn!("gameoverlayrenderer64.dll not found, skipping load");
    }

    Ok(())
}

fn cleanup() {
    let install_path = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags("Software\\WOW6432Node\\Valve\\Steam", winreg::enums::KEY_ALL_ACCESS)
        .and_then(|key| {
            let path = key.get_value::<String, _>("InstallPath")?;
            Ok(PathBuf::from(path))
        });

    if let Ok(path) = install_path {
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Valve\\Steam\\ActiveProcess", winreg::enums::KEY_ALL_ACCESS)
            .map(|key| {
                key.set_value("SteamClientDll64", &path.join("steamclient64.dll").to_str().unwrap()).unwrap();
            }).ok();
        
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Valve\\Steam", winreg::enums::KEY_ALL_ACCESS)
            .map(|key| {
                key.set_value("SteamPath", &path.to_str().unwrap()).unwrap();
            }).ok();
    }

    log::info!("Cleaned up registry");
}
