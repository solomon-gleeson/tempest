use colored::Colorize;
use crate::config::Config;
use crate::TempestError;

const BASE: &str = "https://playvortex.io";

pub async fn login() {
    println!("{}", "=== Vortex Login ===".bold().cyan());

    let username = prompt("Username: ");
    let password = prompt_hidden("Password: ");

    if username.is_empty() || password.is_empty() {
        eprintln!("{} Username and password required.", "[ERROR]".red());
        return;
    }

    println!("{} Signing in...", "[INFO]".cyan());
    match login_direct(&username, &password).await {
        Ok(token) => {
            let mut cfg = Config::load();
            cfg.auth.session_token = Some(token);
            cfg.auth.username = Some(username.clone());
            if let Err(e) = cfg.save() {
                eprintln!("{} Failed to save config: {}", "[ERROR]".red(), e);
            } else {
                println!("{} Logged in as {}", "[DONE]".green(), username.bold());
            }
        }
        Err(e) => eprintln!("{} Login failed: {}", "[ERROR]".red(), e),
    }
}

pub async fn login_direct(username: &str, password: &str) -> Result<String, TempestError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let resp = client
        .post(format!("{BASE}/login"))
        .form(&[
            ("username", username),
            ("password", password),
            ("fingerprint", ""),
            ("fp_token", ""),
        ])
        .send()
        .await?;

    let status = resp.status();
    if status.is_redirection() || status.is_success() {
        if let Some(cookie) = resp.cookies().find(|c| c.name() == "session_token") {
            return Ok(cookie.value().to_string());
        }
        return Err(TempestError::AuthError(
            "server accepted login but set no session_token cookie".to_string(),
        ));
    }

    let body = resp.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v["detail"].as_str().map(str::to_string))
        .unwrap_or_else(|| "invalid username or password".to_string());
    Err(TempestError::AuthError(detail))
}

pub async fn get_play_uri(session_token: &str, game_id: u32) -> Result<String, TempestError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{BASE}/games/{game_id}/play"))
        .header("Cookie", format!("session_token={session_token}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(TempestError::AuthError(format!(
            "Failed to fetch play page: {}",
            resp.status()
        )));
    }

    let html = resp.text().await?;
    if let Some(start) = html.find("vortex://") {
        let uri = &html[start..];
        let end = uri
            .find(|c: char| c == '"' || c == '\'' || c.is_whitespace())
            .unwrap_or(uri.len());
        return Ok(uri[..end].to_string());
    }

    Err(TempestError::AuthError(
        "Could not find vortex:// URI in play page. Is the session token valid?".to_string(),
    ))
}

fn prompt(label: &str) -> String {
    use std::io::Write;
    print!("{} {}", ">>>".cyan(), label);
    std::io::stdout().flush().ok();
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    buf.trim().to_string()
}

fn prompt_hidden(label: &str) -> String {
    use std::io::{BufRead, Write};
    print!("{} {}", ">>>".cyan(), label);
    std::io::stdout().flush().ok();

    let fd = 0;
    let mut term = unsafe { std::mem::zeroed::<libc::termios>() };
    let have_term = unsafe { libc::tcgetattr(fd, &mut term) } == 0;
    let restore = term;
    if have_term {
        term.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &term) };
    }

    let mut buf = String::new();
    std::io::stdin().lock().read_line(&mut buf).ok();

    if have_term {
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &restore) };
        println!();
    }
    buf.trim().to_string()
}
