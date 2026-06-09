use colored::Colorize;
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

    match fetch_games(&token).await {
        Ok(games) => {
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
                let status = if game.installed {
                    "[INSTALLED]".green()
                } else {
                    "[AVAILABLE]".cyan()
                };
                println!("  {:>5}  {}  {}", game.id.to_string().bold(), status, game.name.bold());
            }

            println!();
            println!("  Run {} to launch a game.", "tempest play <id>".cyan());
        }
        Err(e) => {
            crate::logger::error(&format!("Failed to fetch games: {}", e));
            println!("{} Failed to fetch games: {}", "[ERROR]".red(), e);
        }
    }
}

struct Game {
    id: u32,
    name: String,
    installed: bool,
}

async fn fetch_games(token: &str) -> Result<Vec<Game>, TempestError> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{BASE}/games"))
        .header("Cookie", format!("session_token={}", token))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(TempestError::NetworkError(resp.error_for_status().unwrap_err()));
    }

    let body = resp.text().await?;

    let games = parse_games_from_html(&body);
    if !games.is_empty() {
        return Ok(games);
    }

    let games = parse_games_from_json(&body)?;
    Ok(games)
}

fn parse_games_from_html(html: &str) -> Vec<Game> {
    let mut games = Vec::new();
    let mut pos = 0;

    while let Some(start) = html[pos..].find("/games/") {
        let entry_start = pos + start + 7;
        let after_slash = &html[entry_start..];
        let end = after_slash
            .find(|c: char| c == '/' || c == '"' || c == '\'' || c.is_whitespace())
            .unwrap_or(after_slash.len());
        let id_str = &after_slash[..end];

        if let Ok(id) = id_str.parse::<u32>() {
            let name = extract_game_name(html, entry_start + end).unwrap_or_else(|| format!("Game {}", id));
            if !games.iter().any(|g: &Game| g.id == id) {
                games.push(Game {
                    id,
                    name,
                    installed: false,
                });
            }
        }

        pos = entry_start + end;
    }

    games
}

fn extract_game_name(html: &str, around: usize) -> Option<String> {
    let window_start = around.saturating_sub(200);
    let window_end = (around + 200).min(html.len());
    let window = &html[window_start..window_end];

    for tag in &["<h2", "<h3", "<title", "alt=\"", "data-name=\""] {
        let mut search_pos = 0;
        while let Some(tag_start) = window[search_pos..].find(tag) {
            let abs_pos = search_pos + tag_start;
            let after = &window[abs_pos..];

            if let Some(val_start) = after.find('>') {
                let content = &after[val_start + 1..];
                let val_end = content
                    .find(['<', '"'])
                    .unwrap_or(content.len());
                let name = content[..val_end].trim();
                if !name.is_empty()
                    && name.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '\'' || c == ':')
                {
                    if name.len() > 100 {
                        search_pos = abs_pos + 1;
                        continue;
                    }
                    return Some(name.to_string());
                }
            }
            search_pos = abs_pos + 1;
        }
    }
    None
}

fn parse_games_from_json(body: &str) -> Result<Vec<Game>, TempestError> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| TempestError::Other("Could not parse games response".to_string()))?;

    let arr = match json.as_array() {
        Some(a) => a,
        None => {
            if let Some(data) = json.get("data").and_then(|v| v.as_array()) {
                data
            } else if let Some(games) = json.get("games").and_then(|v| v.as_array()) {
                games
            } else {
                return Err(TempestError::Other(
                    "Unexpected JSON structure for games".to_string(),
                ));
            }
        }
    };

    let games = arr
        .iter()
        .filter_map(|item| {
            let id = item
                .get("id")
                .and_then(|v| v.as_u64())
                .or_else(|| item.get("game_id").and_then(|v| v.as_u64()))? as u32;
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("title").and_then(|v| v.as_str()))?
                .to_string();
            let installed = item
                .get("installed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            Some(Game {
                id,
                name,
                installed,
            })
        })
        .collect();

    Ok(games)
}
