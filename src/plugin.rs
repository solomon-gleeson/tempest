use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use crate::config::Config;

const FPS_UNLOCKER_C: &[u8] = include_bytes!("../plugins/fps-unlocker/present_mode_layer.c");
const FPS_UNLOCKER_JSON: &[u8] =
    include_bytes!("../plugins/fps-unlocker/VkLayer_vortstrap_present_mode.json");

fn plugin_dir(name: &str) -> PathBuf {
    Config::data_dir().join("plugins").join(name)
}

fn installed_plugins() -> Vec<String> {
    let plugins_dir = Config::data_dir().join("plugins");
    if !plugins_dir.is_dir() {
        return vec![];
    }
    let mut plugins = vec![];
    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                plugins.push(name.to_string());
            }
        }
    }
    plugins.sort();
    plugins
}

fn is_installed(name: &str) -> bool {
    plugin_dir(name).join("libVkLayer_vortstrap_present_mode.so").exists()
        && plugin_dir(name).join("VkLayer_vortstrap_present_mode.json").exists()
}

fn install_fps_unlocker() -> Result<(), String> {
    let dir = plugin_dir("fps-unlocker");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create plugin dir: {}", e))?;

    let tmp = std::env::temp_dir().join("tempest-fps-unlocker");
    std::fs::create_dir_all(&tmp)
        .map_err(|e| format!("failed to create temp dir: {}", e))?;

    let c_path = tmp.join("present_mode_layer.c");
    let so_path = tmp.join("libVkLayer_vortstrap_present_mode.so");

    std::fs::write(&c_path, FPS_UNLOCKER_C)
        .map_err(|e| format!("failed to write C source: {}", e))?;

    let status = Command::new("cc")
        .args([
            "-I/usr/include",
            "-shared", "-fPIC", "-O2", "-fvisibility=hidden",
            "-Wall", "-Wextra",
            "-o",
        ])
        .arg(&so_path)
        .arg(&c_path)
        .status()
        .map_err(|e| format!("failed to run compiler: {}", e))?;

    if !status.success() {
        std::fs::remove_dir_all(&tmp).ok();
        return Err("compilation failed".to_string());
    }

    std::fs::copy(&so_path, dir.join("libVkLayer_vortstrap_present_mode.so"))
        .map_err(|e| format!("failed to copy .so: {}", e))?;

    std::fs::write(dir.join("VkLayer_vortstrap_present_mode.json"), FPS_UNLOCKER_JSON)
        .map_err(|e| format!("failed to write manifest: {}", e))?;

    std::fs::remove_dir_all(&tmp).ok();
    Ok(())
}

pub fn run(args: &[String]) {
    match args {
        [] => list_plugins(),
        [name] => {
            match name.as_str() {
                "fps-unlocker" => {
                    match install_fps_unlocker() {
                        Ok(()) => println!("Installed fps-unlocker plugin."),
                        Err(e) => eprintln!("Failed to install fps-unlocker: {}", e),
                    }
                }
                _ => eprintln!("Unknown plugin '{}'. Available: fps-unlocker", name),
            }
        }
        _ => eprintln!("Usage: tempest plugin [<name>]"),
    }
}

fn list_plugins() {
    let installed = installed_plugins();
    if installed.is_empty() {
        println!("No plugins installed.");
        println!("  Available: fps-unlocker");
        println!("  Run `tempest plugin <name>` to install.");
        return;
    }
    for name in &installed {
        print!("  {} ", name);
        let ok = match name.as_str() {
            "fps-unlocker" => is_installed(name),
            _ => false,
        };
        if ok {
            println!("[installed]");
        } else {
            println!("[incomplete]");
        }
    }
    println!("  Run `tempest plugin <name>` to install.");
}

pub fn env_vars(_config: &Config) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    let plugins_dir = Config::data_dir().join("plugins");
    for name in installed_plugins() {
        if name.as_str() == "fps-unlocker" {
            let dir = plugins_dir.join("fps-unlocker");
            if dir.join("libVkLayer_vortstrap_present_mode.so").exists() {
                vars.insert(
                    "VK_ADD_IMPLICIT_LAYER_PATH".to_string(),
                    dir.to_string_lossy().to_string(),
                );
                vars.insert("VORTSTRAP_FORCE_PRESENT".to_string(), "1".to_string());
                vars.insert(
                    "VORTSTRAP_PRESENT_MODE".to_string(),
                    getenv_or("VORTSTRAP_PRESENT_MODE", "0"),
                );
            }
        }
    }
    vars
}

fn getenv_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
