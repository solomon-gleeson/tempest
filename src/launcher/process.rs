use colored::Colorize;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use crate::config::Config;

pub struct ProcessManager {
    receiver: Option<Child>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self { receiver: None }
    }

    pub fn receiver_path(cfg: &Config) -> PathBuf {
        cfg.paths.vortex_exe
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("receiver.exe")
    }

    pub fn ensure_receiver(&mut self, cfg: &Config) {
        let path = Self::receiver_path(cfg);

        if !path.exists() {
            println!(
                "{} receiver.exe not found at {} (in-game notifications may not work)",
                "[WARN]".yellow(),
                path.display()
            );
            return;
        }

        if is_receiver_running() {
            tracing::debug!("receiver.exe already running");
            return;
        }

        println!("{} Starting receiver.exe...", "[INFO]".cyan());
        match Command::new(&cfg.wine.binary)
            .env("WINEPREFIX", &cfg.paths.wine_prefix)
            .env("WINEDEBUG", "-all")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                println!("{} receiver.exe started (pid {})", "[PASS]".green(), child.id());
                self.receiver = Some(child);
            }
            Err(e) => {
                println!("{} Could not start receiver.exe: {}", "[WARN]".yellow(), e);
            }
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.receiver.take() {
            tracing::debug!("Stopping receiver.exe");
            child.kill().ok();
            child.wait().ok();
        }
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn is_receiver_running() -> bool {
    std::process::Command::new("pgrep")
        .args(["-f", "receiver.exe"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
