use colored::Colorize;
use crate::config::Config;
use crate::TempestError;

pub async fn login() {
    println!("{}", "=== Vortex Login ===".bold().cyan());

    std::process::Command::new("xdg-open")
        .arg("https://vortex.towerstats.com/")
        .spawn()
        .ok();

    println!("{} Log in to Vortex in your browser.", "[INFO]".cyan());
    println!("{} Then paste your session_token cookie here:", "[INFO]".cyan());
    println!("{}", "  Firefox: F12 -> Storage -> Cookies -> vortex.towerstats.com -> session_token".italic());
    println!("{}", "  Chrome:  F12 -> Application -> Cookies -> vortex.towerstats.com -> session_token".italic());
    println!();
    print!("{} Token: ", ">>>".cyan());

    use std::io::Write;
    std::io::stdout().flush().ok();

    let mut token = String::new();
    std::io::stdin().read_line(&mut token).unwrap();
    let token = token.trim().to_string();

    if token.is_empty() {
        eprintln!("{} No token provided.", "[ERROR]".red());
        return;
    }

    println!("{} Validating token...", "[INFO]".cyan());
    match validate_token(&token).await {
        Ok(username) => {
            println!("{} Logged in as {}", "[PASS]".green(), username.bold());
            let mut cfg = Config::load();
            cfg.auth.session_token = Some(token);
            cfg.auth.username = Some(username);
            if let Err(e) = cfg.save() {
                eprintln!("{} Failed to save config: {}", "[ERROR]".red(), e);
            } else {
                println!("{} Token saved to config.", "[DONE]".green());
            }
        }
        Err(e) => {
            eprintln!("{} Token validation failed: {}", "[ERROR]".red(), e);
        }
    }
}

async fn validate_token(token: &str) -> Result<String, TempestError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let resp = client
        .get("https://vortex.towerstats.com/")
        .header("Cookie", format!("session_token={}", token))
        .send()
        .await?;

    let status = resp.status();

    if status.is_redirection() {
        let location = resp.headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if location.contains("login") || location.contains("signin") {
            return Err(TempestError::AuthError("Token rejected by server".to_string()));
        }
    }

    if !status.is_success() && !status.is_redirection() {
        return Err(TempestError::AuthError(format!("Server returned {}", status)));
    }

    let html = resp.text().await.unwrap_or_default();
    let username = extract_username_from_html(&html)
        .unwrap_or_else(|| "player".to_string());

    Ok(username)
}

fn extract_username_from_html(html: &str) -> Option<String> {
    for pattern in &[r#"data-username=""#, r#""username":""#, r#"data-user=""#] {
        if let Some(start) = html.find(pattern) {
            let rest = &html[start + pattern.len()..];
            if let Some(end) = rest.find(|c: char| c == '"' || c == '<') {
                let name = rest[..end].trim().to_string();
                if !name.is_empty() && name.len() < 64 {
                    return Some(name);
                }
            }
        }
    }
    None
}

pub async fn get_play_uri(session_token: &str, game_id: u32) -> Result<String, TempestError> {
    let client = reqwest::Client::new();
    let url = format!("https://vortex.towerstats.com/games/{}/play", game_id);
    let resp = client
        .get(&url)
        .header("Cookie", format!("session_token={}", session_token))
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
        let uri_part = &html[start..];
        let end = uri_part
            .find(|c: char| c == '"' || c == '\'' || c.is_whitespace())
            .unwrap_or(uri_part.len());
        let uri = uri_part[..end].to_string();
        tracing::debug!("Play URI: {}", uri);
        return Ok(uri);
    }

    Err(TempestError::AuthError(
        "Could not find vortex:// URI in play page — is the session token valid?".to_string(),
    ))
}
