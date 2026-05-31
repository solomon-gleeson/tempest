use colored::Colorize;
use std::process::Command;
use url::Url;

pub fn parse_vortex_uri(uri: &str) -> Option<(u32, String)> {
    let parsed = Url::parse(uri).ok()?;
    if parsed.scheme() != "vortex" {
        return None;
    }
    let mut game_id = None;
    let mut token = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "game" => game_id = value.parse::<u32>().ok(),
            "token" => token = Some(value.into_owned()),
            _ => {}
        }
    }
    Some((game_id?, token?))
}

pub fn register() {
    let exe_path = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("/usr/local/bin/tempest"));

    let apps_dir = dirs::data_local_dir()
        .unwrap_or_default()
        .join("applications");
    std::fs::create_dir_all(&apps_dir).ok();

    let desktop_path = apps_dir.join("tempest-vortex.desktop");
    let contents = format!(
        "[Desktop Entry]\n\
         Name=Tempest (Vortex Launcher)\n\
         Exec={} uri-handler %u\n\
         Type=Application\n\
         MimeType=x-scheme-handler/vortex;\n\
         NoDisplay=true\n",
        exe_path.display()
    );

    if let Err(e) = std::fs::write(&desktop_path, &contents) {
        eprintln!("{} Failed to write .desktop file: {}", "[ERROR]".red(), e);
        return;
    }

    Command::new("xdg-mime")
        .args(["default", "tempest-vortex.desktop", "x-scheme-handler/vortex"])
        .status()
        .ok();

    Command::new("gio")
        .args(["mime", "x-scheme-handler/vortex", "tempest-vortex.desktop"])
        .status()
        .ok();

    Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status()
        .ok();

    println!("{} Registered vortex:// URI handler", "[PASS]".green());
}

pub async fn handle(uri: &str) {
    tracing::debug!("Handling URI: {}", uri);
    match parse_vortex_uri(uri) {
        Some((game_id, token)) => {
            println!("{} Launching game {} via URI", "[INFO]".cyan(), game_id);
            crate::launcher::play_with_token(game_id, token).await;
        }
        None => {
            eprintln!("{} Invalid vortex:// URI: {}", "[ERROR]".red(), uri);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_uri() {
        let result = parse_vortex_uri("vortex://play?game=4&token=abc123");
        assert_eq!(result, Some((4, "abc123".to_string())));
    }

    #[test]
    fn parse_invalid_scheme() {
        assert!(parse_vortex_uri("http://example.com").is_none());
    }

    #[test]
    fn parse_missing_token() {
        assert!(parse_vortex_uri("vortex://play?game=4").is_none());
    }
}
