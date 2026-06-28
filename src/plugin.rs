use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use crate::config::Config;

const FPS_UNLOCKER_C: &[u8] = include_bytes!("../plugins/fps-unlocker/present_mode_layer.c");
const FPS_UNLOCKER_JSON: &[u8] =
    include_bytes!("../plugins/fps-unlocker/VkLayer_vortstrap_present_mode.json");
const OPTIMIZER_C: &[u8] = include_bytes!("../plugins/vortex-optim/optimizer.c");

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
    match name {
        "fps-unlocker" =>
            plugin_dir("fps-unlocker").join("libVkLayer_vortstrap_present_mode.so").exists()
            && plugin_dir("fps-unlocker").join("VkLayer_vortstrap_present_mode.json").exists(),
        "vortex-optim" =>
            plugin_dir("vortex-optim").join("vortex-optim").exists(),
        _ => false,
    }
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

fn install_vortex_optim() -> Result<(), String> {
    let dir = plugin_dir("vortex-optim");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create plugin dir: {}", e))?;

    let tmp = std::env::temp_dir().join("tempest-vortex-optim");
    std::fs::create_dir_all(&tmp)
        .map_err(|e| format!("failed to create temp dir: {}", e))?;

    let c_path = tmp.join("optimizer.c");
    let bin_path = tmp.join("vortex-optim");

    std::fs::write(&c_path, OPTIMIZER_C)
        .map_err(|e| format!("failed to write C source: {}", e))?;

    let status = Command::new("cc")
        .args([
            "-O2", "-std=c11", "-Wall", "-Wextra", "-o",
        ])
        .arg(&bin_path)
        .arg(&c_path)
        .status()
        .map_err(|e| format!("failed to run compiler: {}", e))?;

    if !status.success() {
        std::fs::remove_dir_all(&tmp).ok();
        return Err("compilation failed".to_string());
    }

    std::fs::copy(&bin_path, dir.join("vortex-optim"))
        .map_err(|e| format!("failed to copy binary: {}", e))?;

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
                "vortex-optim" => {
                    match install_vortex_optim() {
                        Ok(()) => println!("Installed vortex-optim plugin."),
                        Err(e) => eprintln!("Failed to install vortex-optim: {}", e),
                    }
                }
                _ => eprintln!("Unknown plugin '{}'. Available: fps-unlocker, vortex-optim", name),
            }
        }
        [verb, name] if verb == "uninstall" => {
            let dir = plugin_dir(name);
            if dir.is_dir() {
                match std::fs::remove_dir_all(&dir) {
                    Ok(()) => println!("Removed plugin '{}'.", name),
                    Err(e) => eprintln!("Failed to remove plugin '{}': {}", name, e),
                }
            } else {
                eprintln!("Plugin '{}' is not installed.", name);
            }
        }
        _ => eprintln!("Usage: tempest plugin [<name>] | tempest plugin uninstall <name>"),
    }
}

fn list_plugins() {
    let installed = installed_plugins();
    if installed.is_empty() {
        println!("No plugins installed.");
        println!("  Available: fps-unlocker, vortex-optim");
        println!("  Run `tempest plugin <name>` to install.");
        return;
    }
    for name in &installed {
        print!("  {} ", name);
        let ok = is_installed(name);
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
        match name.as_str() {
            "fps-unlocker" => {
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
            "vortex-optim" if is_installed("vortex-optim") => {
                vars.insert("DXVK_STATE_CACHE".to_string(), "1".to_string());
                vars.insert("mesa_glthread".to_string(), "true".to_string());
                vars.insert("MESA_NO_DITHER".to_string(), "1".to_string());
                let dxvk = getenv_or("DXVK_CONFIG",
                    "dxvk.enableAsync=true,dxvk.numCompilerThreads=2");
                if !dxvk.contains("dxvk.enableAsync") {
                    vars.insert("DXVK_CONFIG".to_string(), format!(
                        "{},dxvk.enableAsync=true,dxvk.numCompilerThreads=2", dxvk));
                } else {
                    vars.insert("DXVK_CONFIG".to_string(), dxvk);
                }
            }
            _ => {}
        }
    }
    vars
}

pub fn installed(name: &str) -> bool {
    installed_plugins().iter().any(|p| p == name) && is_installed(name)
}

pub fn binary_path(name: &str) -> Option<PathBuf> {
    if !installed(name) { return None; }
    let p = plugin_dir(name).join(name);
    if p.is_file() { Some(p) } else { None }
}

fn getenv_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
