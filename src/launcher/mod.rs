pub mod process;

use colored::Colorize;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use crate::config::Config;

fn perf_summary(config: &Config, use_gamemode: bool) -> String {
    let mut active = vec![];
    if config.launcher.use_fsync { active.push("fsync"); }
    else if config.launcher.use_esync { active.push("esync"); }
    if use_gamemode { active.push("gamemode"); }
    if config.launcher.shader_cache { active.push("shader-cache"); }
    if active.is_empty() { "none".to_string() } else { active.join(" ") }
}

const NOISE_PATTERNS: &[&str] = &[
    "fixme:",
    "libEGL warning",
    "pci id for fd",
    "wine-staging",
    "experimental patches",
    "DxgiFactory::QueryInterface",
    "DxgiAdapter::QueryInterface",
    "create_factory_media",
    "EnableNonClientDpiScaling",
    "DwmSetWindowAttribute",
];

fn is_noise(line: &str) -> bool {
    NOISE_PATTERNS.iter().any(|p| line.contains(p))
}

fn build_wine_command(config: &Config, uri: &str, use_gamemode: bool) -> Command {
    let perf = &config.launcher;

    let mut cmd = if use_gamemode {
        let mut c = Command::new("gamemoderun");
        c.arg(&config.wine.binary);
        c
    } else {
        Command::new(&config.wine.binary)
    };

    cmd.env("WINEPREFIX", &config.paths.wine_prefix);

    cmd.env("WGPU_BACKEND", "vulkan");

    if perf.use_esync { cmd.env("WINEESYNC", "1"); }
    if perf.use_fsync { cmd.env("WINEFSYNC", "1"); }

    if perf.shader_cache {
        let cache = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from(
                std::env::var("HOME").unwrap_or_default() + "/.cache"
            ))
            .join("vortex-shaders");
        std::fs::create_dir_all(&cache).ok();
        cmd.env("VKD3D_SHADER_CACHE_PATH", cache);
    }

    for (key, value) in &config.wine.env {
        cmd.env(key, value);
    }

    cmd.arg(&config.paths.vortex_exe);
    cmd.arg(uri);
    cmd
}

pub async fn play(game_id: u32) {
    let mut cfg = Config::load();

    let token = match cfg.auth.session_token.clone() {
        Some(t) => t,
        None => {
            println!("{} Not logged in. Running login flow...", "[WARN]".yellow());
            crate::auth::login().await;
            cfg = Config::load();
            match cfg.auth.session_token.clone() {
                Some(t) => t,
                None => {
                    eprintln!("{} Login failed. Aborting.", "[ERROR]".red());
                    return;
                }
            }
        }
    };

    println!("{} Fetching play URI for game {}...", "[INFO]".cyan(), game_id);
    match crate::auth::get_play_uri(&token, game_id).await {
        Ok(uri) => launch_with_uri(uri).await,
        Err(e) => {
            eprintln!("{} Failed to get play URI: {}", "[ERROR]".red(), e);
            eprintln!("  Try {} to re-authenticate.", "tempest login".cyan());
        }
    }
}

pub async fn play_with_token(game_id: u32, token: String) {
    let uri = format!("vortex://play?game={}&token={}", game_id, token);
    launch_with_uri(uri).await;
}

async fn launch_with_uri(uri: String) {
    let cfg = Config::load();

    if !cfg.paths.vortex_exe.exists() {
        eprintln!("{} Vortex.exe not found at {}", "[ERROR]".red(), cfg.paths.vortex_exe.display());
        eprintln!("  Run {} first.", "tempest setup".cyan());
        return;
    }

    let game_id = crate::uri::parse_vortex_uri(&uri)
        .map(|(id, _)| id.to_string())
        .unwrap_or_else(|| "?".to_string());

    let use_gamemode = cfg.launcher.use_gamemode && which::which("gamemoderun").is_ok();
    println!("{} Launching Vortex for game {} [{}]...",
        "[INFO]".cyan(), game_id, perf_summary(&cfg, use_gamemode).bold());
    tracing::debug!("Launch URI: {}", uri);

    let mut pm = process::ProcessManager::new();
    pm.ensure_receiver(&cfg);

    let mut cmd = build_wine_command(&cfg, &uri, use_gamemode);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Failed to launch Wine: {}", "[ERROR]".red(), e);
            eprintln!("  Is Wine installed? Try {} for diagnostics.", "tempest doctor".cyan());
            return;
        }
    };

    let child_id = child.id();
    ctrlc::set_handler(move || {
        unsafe { libc::kill(child_id as i32, libc::SIGTERM); }
    }).ok();

    let stderr = child.stderr.take().map(BufReader::new);
    let stdout = child.stdout.take().map(BufReader::new);
    let filter = cfg.launcher.filter_wine_noise;

    let stderr_handle = std::thread::spawn(move || {
        if let Some(reader) = stderr {
            for line in reader.lines().map_while(Result::ok) {
                if filter && is_noise(&line) { continue; }
                eprintln!("{}", line);
            }
        }
    });

    let stdout_handle = std::thread::spawn(move || {
        if let Some(reader) = stdout {
            for line in reader.lines().map_while(Result::ok) {
                if filter && is_noise(&line) { continue; }
                println!("{}", line);
            }
        }
    });

    let status = child.wait().unwrap_or_else(|_| std::process::exit(1));
    stderr_handle.join().ok();
    stdout_handle.join().ok();

    drop(pm);

    match status.code() {
        Some(0) => println!("{} Game exited cleanly.", "[DONE]".green()),
        Some(code) => {
            eprintln!("{} Wine exited with code {}.", "[WARN]".yellow(), code);
            eprintln!("  Run {} for diagnostics.", "tempest doctor".cyan());
        }
        None => {
            eprintln!("{} Wine process was terminated by a signal.", "[WARN]".yellow());
            eprintln!("  This may indicate a crash. Check Wine compatibility.");
        }
    }
}
