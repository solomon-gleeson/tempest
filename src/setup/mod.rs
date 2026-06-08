pub mod dll;
pub mod dxvk;
pub mod vkd3d;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write as _;
use std::path::Path;
use std::process::Command;
use crate::config::Config;
use crate::TempestError;

pub enum Distro {
    Fedora,
    Debian,
    Arch,
    OpenSuse,
    Unknown(String),
}

impl std::fmt::Display for Distro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Distro::Fedora    => write!(f, "Fedora/RHEL"),
            Distro::Debian    => write!(f, "Debian/Ubuntu"),
            Distro::Arch      => write!(f, "Arch Linux"),
            Distro::OpenSuse  => write!(f, "openSUSE"),
            Distro::Unknown(s) => write!(f, "Unknown ({})", s),
        }
    }
}

pub fn detect_distro() -> Distro {
    let contents = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut id = String::new();
    let mut id_like = String::new();
    for line in contents.lines() {
        if let Some(val) = line.strip_prefix("ID=") {
            id = val.trim_matches('"').to_lowercase();
        } else if let Some(val) = line.strip_prefix("ID_LIKE=") {
            id_like = val.trim_matches('"').to_lowercase();
        }
    }
    let check = |s: &str| id == s || id_like.contains(s);
    if check("fedora") || check("rhel") || check("centos") { Distro::Fedora }
    else if check("debian") || check("ubuntu")             { Distro::Debian }
    else if check("arch")                                  { Distro::Arch }
    else if check("opensuse") || check("suse")             { Distro::OpenSuse }
    else                                                   { Distro::Unknown(id) }
}


pub(crate) async fn fetch_github_release(
    client: &reqwest::Client,
    repo: &str,
    asset_suffix: &str,
) -> Result<(String, String), TempestError> {
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let body: serde_json::Value = client
        .get(&api_url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?
        .json()
        .await?;

    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| TempestError::Other("No tag_name in GitHub release".into()))?
        .to_string();

    let download_url = body["assets"]
        .as_array()
        .ok_or_else(|| TempestError::Other("No assets in GitHub release".into()))?
        .iter()
        .find_map(|a| {
            let name = a["name"].as_str()?;
            if name.ends_with(asset_suffix) {
                a["browser_download_url"].as_str().map(str::to_string)
            } else {
                None
            }
        })
        .ok_or_else(|| TempestError::Other(
            format!("No asset with suffix '{}' in release {}", asset_suffix, tag)
        ))?;

    Ok((tag, download_url))
}

pub(crate) fn progress_bar(total: Option<u64>) -> ProgressBar {
    let pb = ProgressBar::new(total.unwrap_or(0));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb
}

pub(crate) async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<(), TempestError> {
    use futures_util::StreamExt;

    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(TempestError::Other(format!("Download failed: {}", resp.status())));
    }

    let pb = progress_bar(resp.content_length());

    let mut file = std::fs::File::create(dest)?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(TempestError::NetworkError)?;
        file.write_all(&chunk)?;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("done");
    Ok(())
}


struct DistroCommands {
    update:        &'static str,
    wine_packages: &'static str,
    extra_setup:   Option<&'static str>,
    winetricks_manual: bool,
}

fn distro_commands(distro: &Distro) -> DistroCommands {
    match distro {
        Distro::Fedora => DistroCommands {
            update:        "sudo dnf upgrade --refresh",
            wine_packages: "sudo dnf install wine winetricks wine.i686",
            extra_setup:   None,
            winetricks_manual: false,
        },
        Distro::Debian => DistroCommands {
            update:        "sudo apt update && sudo apt upgrade",
            wine_packages: "sudo apt install wine64 wine32",
            extra_setup:   Some("sudo dpkg --add-architecture i386 && sudo apt update"),
            winetricks_manual: true,
        },
        Distro::Arch => DistroCommands {
            update:        "sudo pacman -Syu",
            wine_packages: "sudo pacman -S wine winetricks",
            extra_setup:   None,
            winetricks_manual: false,
        },
        Distro::OpenSuse => DistroCommands {
            update:        "sudo zypper refresh && sudo zypper update",
            wine_packages: "sudo zypper install wine winetricks wine-32bit",
            extra_setup:   None,
            winetricks_manual: false,
        },
        Distro::Unknown(_) => DistroCommands {
            update:        "# Update your system",
            wine_packages: "# Install wine and winetricks",
            extra_setup:   None,
            winetricks_manual: false,
        },
    }
}

pub(crate) fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

fn run_cmd(cmd: &str) -> bool {
    let effective = if is_root() {
        cmd.replace("sudo ", "")
    } else {
        cmd.to_string()
    };
    println!("{} {}", ">>>".cyan(), effective.bold());
    Command::new("sh")
        .arg("-c")
        .arg(&effective)
        .status()
        .map(|s| s.success())
        .unwrap_or_else(|e| { eprintln!("{} {}", "[ERROR]".red(), e); false })
}

fn prompt_confirm(msg: &str) -> bool {
    print!("{} [y/N] ", msg.yellow());
    std::io::stdout().flush().ok();
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
}


pub async fn run() {
    println!("{}", "=== Tempest Setup ===".bold().cyan());
    println!();

    let distro = detect_distro();
    println!("{} Detected distro: {}", "[INFO]".cyan(), distro.to_string().bold());

    let cmds = distro_commands(&distro);
    if which::which("wine").is_ok() {
        println!("{} Wine already installed.", "[PASS]".green());
    } else {
        println!("{} Wine not found. Will install.", "[WARN]".yellow());
        println!();
        if let Some(extra) = cmds.extra_setup { println!("  {}", extra); }
        println!("  {}", cmds.update);
        println!("  {}", cmds.wine_packages);
        println!();

        if prompt_confirm("Proceed with installation?") {
            if let Some(extra) = cmds.extra_setup { run_cmd(extra); }
            run_cmd(cmds.update);
            run_cmd(cmds.wine_packages);
            if cmds.winetricks_manual {
                println!("{} Downloading winetricks...", "[INFO]".cyan());
                run_cmd("sudo curl -L https://raw.githubusercontent.com/Winetricks/winetricks/master/src/winetricks -o /usr/local/bin/winetricks");
                run_cmd("sudo chmod +x /usr/local/bin/winetricks");
            }
        } else {
            println!("{} Skipping Wine installation.", "[WARN]".yellow());
        }
    }

    let cfg = Config::default();
    let prefix = &cfg.paths.wine_prefix;

    println!();
    println!("{} Creating Wine prefix at {}", "[INFO]".cyan(), prefix.display());
    std::fs::create_dir_all(prefix).ok();
    run_cmd(&format!("WINEPREFIX=\"{}\" WINEDEBUG=-all wine wineboot --init", prefix.display()));

    println!("{} Installing winetricks components...", "[INFO]".cyan());
    run_cmd(&format!(
        "WINEPREFIX=\"{}\" WINEDEBUG=-all winetricks -q d3dcompiler_47 vcrun2022 corefonts",
        prefix.display()
    ));

    if which::which("gamemoderun").is_err() {
        let gm_cmd = match &distro {
            Distro::Fedora   => Some("sudo dnf install gamemode"),
            Distro::Debian   => Some("sudo apt install gamemode"),
            Distro::Arch     => Some("sudo pacman -S gamemode"),
            Distro::OpenSuse => Some("sudo zypper install gamemode"),
            _                => None,
        };
        if let Some(cmd) = gm_cmd {
            println!();
            if prompt_confirm("Install GameMode? (reduces latency, sets CPU to performance governor)") {
                run_cmd(cmd);
            }
        }
    } else {
        println!("{} GameMode already installed.", "[PASS]".green());
    }

    println!();
    println!("{} Installing DXVK...", "[INFO]".cyan());
    if let Err(e) = dxvk::install(prefix).await {
        println!("{} DXVK installation failed (non-fatal): {}", "[WARN]".yellow(), e);
    }

    println!();
    println!("{} Installing vkd3d-proton...", "[INFO]".cyan());
    if let Err(e) = vkd3d::install(prefix).await {
        println!("{} vkd3d-proton installation failed (non-fatal): {}", "[WARN]".yellow(), e);
    }

    println!();
    println!("{} Downloading Vortex...", "[INFO]".cyan());
    crate::updater::update().await;

    println!();
    println!("{} Registering vortex:// URI scheme...", "[INFO]".cyan());
    crate::uri::register();

    println!();
    println!("{} Running diagnostics...", "[INFO]".cyan());
    crate::doctor::run();

    cfg.save().ok();

    println!();
    println!(
        "{} Setup complete! Run {} to launch a game.",
        "[DONE]".green().bold(),
        "tempest play <game_id>".cyan()
    );
}

pub fn uninstall() {
    let data_dir   = Config::data_dir();
    let config_dir = Config::config_dir();
    let desktop    = dirs::data_local_dir()
        .unwrap_or_default()
        .join("applications/tempest-vortex.desktop");

    println!("{}", "=== Tempest Uninstall ===".bold().red());
    println!("This will remove:");
    println!("  {}", data_dir.display());
    println!("  {}", config_dir.display());
    println!("  {}", desktop.display());

    let mut buf = String::new();
    print!("{}", "Are you sure? [y/N] ".red().bold());
    std::io::stdout().flush().ok();
    std::io::stdin().read_line(&mut buf).ok();
    if !matches!(buf.trim().to_lowercase().as_str(), "y" | "yes") {
        println!("Cancelled.");
        return;
    }

    for path in [&data_dir, &config_dir] {
        if path.exists() {
            std::fs::remove_dir_all(path).ok();
            println!("{} Removed {}", "[DONE]".green(), path.display());
        }
    }
    if desktop.exists() {
        std::fs::remove_file(&desktop).ok();
        println!("{} Removed {}", "[DONE]".green(), desktop.display());
    }

    Command::new("update-desktop-database")
        .arg(dirs::data_local_dir().unwrap_or_default().join("applications"))
        .status()
        .ok();

    println!("{} Uninstall complete.", "[DONE]".green().bold());
}
