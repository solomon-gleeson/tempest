use colored::Colorize;
use std::io::Write;
use crate::config::Config;
use crate::TempestError;

const BASE: &str = "https://playvortex.io";

pub async fn list() {
    println!("{}", "=== Vortex Games ===".bold().cyan());

    let cfg = Config::load();
    let token = match cfg.auth.session_token.clone() {
        Some(t) => t,
        None => {
            println!("{} Not logged in. Run {} first.", "[ERROR]".red(), "tempest login".cyan());
            return;
        }
    };

    let client = reqwest::Client::new();
    let mut games: Vec<Game> = Vec::new();

    for id in 1u32.. {
        print!("\r{} Fetching game {}...", "[INFO]".cyan(), id);
        std::io::stdout().flush().ok();

        match fetch_game_page(&client, &token, id).await {
            Ok(Some(name)) => {
                games.push(Game { id, name });
            }
            Ok(None) => {
                println!();
                break;
            }
            Err(_) => {
                println!();
                break;
            }
        }
    }

    if games.is_empty() {
        println!("{} No games found for your account.", "[INFO]".cyan());
        return;
    }

    println!(
        "{} Found {} game(s)\n",
        "[INFO]".cyan(),
        games.len().to_string().bold()
    );

    for game in &games {
        println!("  {:>5}  {}", game.id.to_string().bold(), game.name.bold());
    }

    println!();
    println!("  Run {} to launch a game.", "tempest play <id>".cyan());
}

struct Game {
    id: u32,
    name: String,
}

async fn fetch_game_page(
    client: &reqwest::Client,
    token: &str,
    id: u32,
) -> Result<Option<String>, TempestError> {
    let resp = client
        .get(format!("{BASE}/games/{id}"))
        .header("Cookie", format!("session_token={}", token))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let body = resp.text().await?;

    let name = extract_game_title(&body);

    Ok(name)
}

fn extract_game_title(html: &str) -> Option<String> {
    let marker = r#"<div class="game-detail-title">"#;
    let start = html.find(marker)?;
    let after_open = &html[start + marker.len()..];
    let end = after_open.find("</div>")?;
    let name = after_open[..end].trim();
    if name.is_empty() { None } else { Some(name.to_string()) }
}
