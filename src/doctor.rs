use colored::Colorize;
use std::net::TcpStream;
use std::time::Duration;
use crate::config::Config;
use crate::setup::detect_distro;
use libc;

struct Check {
    name: &'static str,
    passed: bool,
    detail: String,
    fix: Option<String>,
}

impl Check {
    fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self { name, passed: true, detail: detail.into(), fix: None }
    }

    fn fail(name: &'static str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self { name, passed: false, detail: detail.into(), fix: Some(fix.into()) }
    }

    fn print(&self) {
        if self.passed {
            println!("{} {}: {}", "[PASS]".green().bold(), self.name.bold(), self.detail);
        } else {
            println!("{} {}: {}", "[FAIL]".red().bold(), self.name.bold(), self.detail);
            if let Some(fix) = &self.fix {
                let hint = if unsafe { libc::getuid() == 0 } {
                    fix.replace("sudo ", "")
                } else {
                    fix.clone()
                };
                println!("       {} {}", "-->".yellow(), hint.cyan());
            }
        }
    }
}

pub fn run() {
    println!("{}", "=== Tempest Doctor ===".bold().cyan());
    println!();

    let cfg = Config::load();
    let distro = detect_distro();
    let mut checks = vec![];

    match which::which("wine") {
        Ok(path) => {
            let version = wine_version().unwrap_or_else(|| "unknown".to_string());
            checks.push(Check::pass("Wine installed", format!("{} ({})", path.display(), version)));
        }
        Err(_) => {
            let fix = wine_install_cmd(&distro);
            checks.push(Check::fail("Wine installed", "not found in PATH", fix));
        }
    }

    let prefix_ready = cfg.paths.wine_prefix.join("system.reg").exists();
    if prefix_ready {
        checks.push(Check::pass("Wine prefix exists", cfg.paths.wine_prefix.display().to_string()));
    } else {
        checks.push(Check::fail(
            "Wine prefix exists",
            if cfg.paths.wine_prefix.exists() {
                "directory exists but not initialised (wineboot not run)".to_string()
            } else {
                cfg.paths.wine_prefix.display().to_string()
            },
            "Run: tempest setup",
        ));
    }

    if cfg.paths.vortex_exe.exists() {
        let size = std::fs::metadata(&cfg.paths.vortex_exe)
            .map(|m| format!("{:.1} MB", m.len() as f64 / 1_000_000.0))
            .unwrap_or_default();
        checks.push(Check::pass("Vortex.exe exists", size));
    } else {
        checks.push(Check::fail(
            "Vortex.exe exists",
            "not found",
            "Run: tempest update",
        ));
    }

    let vulkan_status = std::process::Command::new("vulkaninfo")
        .arg("--summary")
        .output();
    match vulkan_status {
        Ok(out) if out.status.success() => {
            let output = String::from_utf8_lossy(&out.stdout);
            let gpu = extract_gpu(&output)
                .unwrap_or_else(|| "GPU detected".to_string());
            let gpu_count = output.lines().filter(|l| l.trim().starts_with("GPU") && l.trim().ends_with(':')).count();
            checks.push(Check::pass("Vulkan working", format!("{} device(s) found", gpu_count)));
            if gpu.to_lowercase().contains("llvmpipe") || gpu.to_lowercase().contains("softpipe") {
                checks.push(Check::fail(
                    "GPU detected",
                    format!("{} (software renderer)", gpu),
                    gpu_driver_fix(&distro),
                ));
            } else {
                checks.push(Check::pass("GPU detected", gpu));
            }
        }
        _ => {
            checks.push(Check::fail(
                "Vulkan working",
                "vulkaninfo failed",
                vulkan_fix(&distro),
            ));
        }
    }

    if is_nvidia() {
        let icd_dir = std::path::Path::new("/usr/share/vulkan/icd.d");
        let nvidia_icd_found = ["nvidia_icd.json", "nvidia_icd.x86_64.json", "nvidia_icd.i686.json"]
            .iter()
            .any(|name| icd_dir.join(name).exists());
        if nvidia_icd_found {
            checks.push(Check::pass("NVIDIA Vulkan ICD", "registered"));
        } else {
            checks.push(Check::fail(
                "NVIDIA Vulkan ICD",
                "not found",
                nvidia_fix(&distro),
            ));
        }
    }

    let uri_check = std::process::Command::new("xdg-mime")
        .args(["query", "default", "x-scheme-handler/vortex"])
        .output();
    match uri_check {
        Ok(out) => {
            let handler = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if handler.contains("tempest") {
                checks.push(Check::pass("URI handler registered", handler));
            } else if handler.is_empty() {
                checks.push(Check::fail(
                    "URI handler registered",
                    "not registered",
                    "Run: tempest setup",
                ));
            } else {
                checks.push(Check::fail(
                    "URI handler registered",
                    format!("wrong handler: {}", handler),
                    "Run: tempest setup",
                ));
            }
        }
        Err(_) => checks.push(Check::fail(
            "URI handler registered",
            "xdg-mime not found",
            "Install xdg-utils",
        )),
    }

    match which::which("winetricks") {
        Ok(path) => checks.push(Check::pass("Winetricks installed", path.display().to_string())),
        Err(_) => checks.push(Check::fail(
            "Winetricks installed",
            "not found",
            winetricks_fix(&distro),
        )),
    }

    if cfg.auth.session_token.is_some() {
        checks.push(Check::pass("Session token stored", ""));
    } else {
        checks.push(Check::fail(
            "Session token stored",
            "not found",
            "Run: tempest login",
        ));
    }

    let tcp_ok = std::net::ToSocketAddrs::to_socket_addrs(&("vortex.towerstats.com", 443u16))
        .ok()
        .and_then(|mut addrs| addrs.next())
        .map(|addr| TcpStream::connect_timeout(&addr, Duration::from_secs(5)).is_ok())
        .unwrap_or(false);
    if tcp_ok {
        checks.push(Check::pass("Network (HTTPS)", "vortex.towerstats.com:443 reachable"));
    } else {
        checks.push(Check::fail(
            "Network (HTTPS)",
            "vortex.towerstats.com:443 unreachable",
            "Check your internet connection or firewall",
        ));
    }

    match which::which("gamemoderun") {
        Ok(_) => {
            let status = if cfg.launcher.use_gamemode { "enabled" } else { "installed (set use_gamemode=true to enable)" };
            checks.push(Check::pass("GameMode", status));
        }
        Err(_) => {
            checks.push(Check::fail(
                "GameMode",
                "not installed (optional but recommended)",
                gamemode_install_cmd(&distro),
            ));
        }
    }

    let dxgi = cfg.paths.wine_prefix.join("drive_c/windows/system32/dxgi.dll");
    if crate::setup::dll::verify_dll(&dxgi) {
        checks.push(Check::pass("DXVK installed", "dxgi.dll is a valid PE"));
    } else {
        checks.push(Check::fail(
            "DXVK installed",
            "dxgi.dll not found or invalid",
            "Run: tempest setup",
        ));
    }

    let d3d12 = cfg.paths.wine_prefix.join("drive_c/windows/system32/d3d12.dll");
    if crate::setup::dll::verify_dll(&d3d12) {
        checks.push(Check::pass("vkd3d-proton installed", "d3d12.dll is a valid PE"));
    } else {
        checks.push(Check::fail(
            "vkd3d-proton installed",
            "d3d12.dll not found or invalid",
            "Run: tempest setup",
        ));
    }

    for check in &checks {
        check.print();
    }

    let failures = checks.iter().filter(|c| !c.passed).count();
    println!();
    if failures == 0 {
        println!("{} All checks passed!", "[DONE]".green().bold());
    } else {
        println!("{} {} check(s) failed.", "[WARN]".yellow().bold(), failures);
    }
}

fn wine_version() -> Option<String> {
    let out = std::process::Command::new("wine")
        .arg("--version")
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn extract_gpu(vulkaninfo: &str) -> Option<String> {
    let mut in_gpu_block = false;
    let mut current_type = String::new();
    let mut current_name = String::new();

    for line in vulkaninfo.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("GPU") && trimmed.ends_with(':') {
            if in_gpu_block && !current_name.is_empty()
                && !current_type.contains("CPU")
                && !current_name.to_lowercase().contains("llvmpipe")
                && !current_name.to_lowercase().contains("softpipe")
            {
                return Some(current_name);
            }
            in_gpu_block = true;
            current_type.clear();
            current_name.clear();
        } else if in_gpu_block {
            if let Some(eq) = trimmed.find('=') {
                let key = trimmed[..eq].trim();
                let val = trimmed[eq + 1..].trim().to_string();
                if key == "deviceName" {
                    current_name = val;
                } else if key == "deviceType" {
                    current_type = val;
                }
            }
        }
    }

    if in_gpu_block && !current_name.is_empty()
        && !current_type.contains("CPU")
        && !current_name.to_lowercase().contains("llvmpipe")
        && !current_name.to_lowercase().contains("softpipe")
    {
        return Some(current_name);
    }

    for line in vulkaninfo.lines() {
        let trimmed = line.trim();
        if let Some(eq) = trimmed.find('=') {
            if trimmed[..eq].trim() == "deviceName" {
                return Some(trimmed[eq + 1..].trim().to_string());
            }
        }
    }
    None
}

fn is_nvidia() -> bool {
    std::path::Path::new("/dev/nvidia0").exists()
        || std::path::Path::new("/proc/driver/nvidia").exists()
}

fn wine_install_cmd(distro: &crate::setup::Distro) -> String {
    match distro {
        crate::setup::Distro::Fedora => "sudo dnf install wine winetricks wine.i686".to_string(),
        crate::setup::Distro::Debian => "sudo apt install wine64 wine32 winetricks".to_string(),
        crate::setup::Distro::Arch => "sudo pacman -S wine winetricks".to_string(),
        crate::setup::Distro::OpenSuse => "sudo zypper install wine winetricks wine-32bit".to_string(),
        crate::setup::Distro::Unknown(_) => "Install wine via your package manager".to_string(),
    }
}

fn winetricks_fix(distro: &crate::setup::Distro) -> String {
    match distro {
        crate::setup::Distro::Fedora => "sudo dnf install winetricks".to_string(),
        crate::setup::Distro::Arch => "sudo pacman -S winetricks".to_string(),
        _ => "curl -L https://raw.githubusercontent.com/Winetricks/winetricks/master/src/winetricks | sudo tee /usr/local/bin/winetricks && sudo chmod +x /usr/local/bin/winetricks".to_string(),
    }
}

fn vulkan_fix(distro: &crate::setup::Distro) -> String {
    match distro {
        crate::setup::Distro::Fedora => "sudo dnf install vulkan-tools mesa-vulkan-drivers".to_string(),
        crate::setup::Distro::Debian => "sudo apt install vulkan-tools mesa-vulkan-drivers".to_string(),
        crate::setup::Distro::Arch => "sudo pacman -S vulkan-tools vulkan-icd-loader".to_string(),
        crate::setup::Distro::OpenSuse => "sudo zypper install vulkan-tools".to_string(),
        crate::setup::Distro::Unknown(_) => "Install vulkan-tools and GPU drivers".to_string(),
    }
}

fn gpu_driver_fix(distro: &crate::setup::Distro) -> String {
    match distro {
        crate::setup::Distro::Fedora => "Install GPU drivers: sudo dnf install mesa-dri-drivers".to_string(),
        crate::setup::Distro::Debian => "Install GPU drivers: sudo apt install mesa-utils".to_string(),
        _ => "Install your GPU's Vulkan driver".to_string(),
    }
}

fn gamemode_install_cmd(distro: &crate::setup::Distro) -> String {
    match distro {
        crate::setup::Distro::Fedora  => "sudo dnf install gamemode".to_string(),
        crate::setup::Distro::Debian  => "sudo apt install gamemode".to_string(),
        crate::setup::Distro::Arch    => "sudo pacman -S gamemode".to_string(),
        crate::setup::Distro::OpenSuse => "sudo zypper install gamemode".to_string(),
        crate::setup::Distro::Unknown(_) => "Install gamemode via your package manager".to_string(),
    }
}

fn nvidia_fix(distro: &crate::setup::Distro) -> String {
    match distro {
        crate::setup::Distro::Fedora => "sudo dnf install nvidia-driver-libs".to_string(),
        crate::setup::Distro::Debian => "sudo apt install nvidia-driver".to_string(),
        crate::setup::Distro::Arch => "sudo pacman -S nvidia-utils".to_string(),
        _ => "Install NVIDIA Vulkan driver libraries".to_string(),
    }
}
