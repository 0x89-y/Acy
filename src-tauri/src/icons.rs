use crate::model::Source;
use crate::sources;
use base64::{engine::general_purpose::STANDARD, Engine};
use regex::Regex;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use tokio::sync::{OnceCell, Semaphore};

static FETCH_SEM: Semaphore = Semaphore::const_new(4);

static CLIENT: OnceCell<Client> = OnceCell::const_new();

const NO_ICON_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

async fn http_client() -> &'static Client {
    CLIENT
        .get_or_init(|| async {
            Client::builder()
                .user_agent(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
                )
                .timeout(Duration::from_secs(8))
                .build()
                .unwrap_or_else(|_| Client::new())
        })
        .await
}

fn source_key(source: Source) -> &'static str {
    match source {
        Source::Winget => "winget",
        Source::Scoop => "scoop",
        Source::Choco => "choco",
        Source::Msstore => "msstore",
        Source::Local => "local",
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn cache_path(app: &AppHandle, source: Source, id: &str) -> Option<PathBuf> {
    let dir = app.path().app_cache_dir().ok()?.join("icons");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join(format!("{}_{}.img", source_key(source), sanitize(id))))
}

fn none_path(app: &AppHandle, source: Source, id: &str) -> Option<PathBuf> {
    let dir = app.path().app_cache_dir().ok()?.join("icons");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join(format!("{}_{}.none", source_key(source), sanitize(id))))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn no_icon_marker_fresh(path: &PathBuf) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(stamp) = text.trim().parse::<u64>() else {
        return false;
    };
    now_secs().saturating_sub(stamp) < NO_ICON_TTL.as_secs()
}

fn mime_of(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.starts_with(b"\x00\x00\x01\x00") {
        "image/x-icon"
    } else if std::str::from_utf8(&bytes[..bytes.len().min(256)])
        .map(|s| {
            let t = s.trim_start();
            t.starts_with("<svg") || t.starts_with("<?xml")
        })
        .unwrap_or(false)
    {
        "image/svg+xml"
    } else {
        "image/png"
    }
}

fn to_data_url(bytes: &[u8]) -> String {
    format!("data:{};base64,{}", mime_of(bytes), STANDARD.encode(bytes))
}

fn pick_icon(html: &str, base: &reqwest::Url) -> Option<reqwest::Url> {
    static LINK: OnceLock<Regex> = OnceLock::new();
    static REL: OnceLock<Regex> = OnceLock::new();
    static HREF: OnceLock<Regex> = OnceLock::new();
    static SIZES: OnceLock<Regex> = OnceLock::new();
    let link = LINK.get_or_init(|| Regex::new(r"(?is)<link\b[^>]*>").unwrap());
    let rel = REL.get_or_init(|| Regex::new(r#"(?is)\brel\s*=\s*["']([^"']+)["']"#).unwrap());
    let href = HREF.get_or_init(|| Regex::new(r#"(?is)\bhref\s*=\s*["']([^"']+)["']"#).unwrap());
    let sizes = SIZES.get_or_init(|| Regex::new(r#"(?is)\bsizes\s*=\s*["']([^"']+)["']"#).unwrap());

    let scope = match html.to_lowercase().find("</head>") {
        Some(end) => &html[..end.min(html.len())],
        None => html,
    };

    let mut best: Option<(i32, String)> = None;
    for m in link.find_iter(scope) {
        let tag = m.as_str();
        let rel_val = rel
            .captures(tag)
            .map(|c| c[1].to_lowercase())
            .unwrap_or_default();
        if !rel_val.contains("icon") || rel_val.contains("mask-icon") {
            continue;
        }
        let Some(href_val) = href.captures(tag).map(|c| c[1].to_string()) else {
            continue;
        };
        if href_val.trim().is_empty() {
            continue;
        }
        let dim = sizes
            .captures(tag)
            .and_then(|c| c[1].split(['x', 'X']).next().map(|s| s.trim().to_string()))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let mut score = dim;
        if rel_val.contains("apple-touch-icon") {
            score += 250;
        }
        if score == 0 {
            score = 1;
        }
        if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, href_val));
        }
    }
    base.join(&best?.1).ok()
}

async fn fetch_bytes(client: &Client, url: reqwest::Url) -> Option<Vec<u8>> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 100 {
        return None;
    }
    Some(bytes.to_vec())
}

async fn favicon_service(domain: &str) -> Option<Vec<u8>> {
    let url = format!("https://www.google.com/s2/favicons?domain={domain}&sz=64");
    let url = reqwest::Url::parse(&url).ok()?;
    let client = http_client().await;
    let mut delay = Duration::from_millis(400);
    for attempt in 0..3 {
        match client.get(url.clone()).send().await {
            Ok(resp) if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
            }
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes().await.ok()?;
                return (bytes.len() >= 100).then(|| bytes.to_vec());
            }
            Ok(_) => return None,
            Err(_) => {}
        }
        if attempt < 2 {
            tokio::time::sleep(delay).await;
            delay *= 2;
        }
    }
    None
}

async fn resolve_icon(source_url: &str) -> Option<Vec<u8>> {
    let base = reqwest::Url::parse(source_url.trim()).ok()?;
    let client = http_client().await;

    if let Ok(resp) = client.get(base.clone()).send().await {
        if resp.status().is_success() {
            let is_image = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|t| t.starts_with("image/"))
                .unwrap_or(false);
            if is_image {
                if let Ok(bytes) = resp.bytes().await {
                    if bytes.len() >= 100 {
                        return Some(bytes.to_vec());
                    }
                }
            } else if let Ok(html) = resp.text().await {
                if let Some(icon_url) = pick_icon(&html, &base) {
                    if let Some(bytes) = fetch_bytes(client, icon_url).await {
                        return Some(bytes);
                    }
                }
            }
        }
    }

    favicon_service(base.host_str()?).await
}

pub async fn get_icon(
    app: &AppHandle,
    source: Source,
    id: String,
    homepage: Option<String>,
) -> Option<String> {
    let path = cache_path(app, source, &id)?;

    if let Ok(bytes) = std::fs::read(&path) {
        if !bytes.is_empty() {
            return Some(to_data_url(&bytes));
        }
    }

    if let Some(none) = none_path(app, source, &id) {
        if no_icon_marker_fresh(&none) {
            return None;
        }
    }

    let _permit = FETCH_SEM.acquire().await.ok()?;

    let homepage = match homepage {
        Some(h) if !h.trim().is_empty() => Some(h),
        _ => sources::for_source(source)
            .info(&id)
            .await
            .ok()
            .flatten()
            .and_then(|p| p.homepage),
    };

    let bytes = match homepage {
        Some(h) => resolve_icon(&h).await,
        None => None,
    };

    match bytes {
        Some(bytes) => {
            let _ = std::fs::write(&path, &bytes);
            if let Some(none) = none_path(app, source, &id) {
                let _ = std::fs::remove_file(none);
            }
            Some(to_data_url(&bytes))
        }
        None => {
            if let Some(none) = none_path(app, source, &id) {
                let _ = std::fs::write(none, now_secs().to_string());
            }
            None
        }
    }
}

pub fn is_resolved(app: &AppHandle, source: Source, id: &str) -> bool {
    let has_icon = cache_path(app, source, id)
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len() > 0)
        .unwrap_or(false);
    if has_icon {
        return true;
    }
    none_path(app, source, id)
        .map(|p| no_icon_marker_fresh(&p))
        .unwrap_or(false)
}

pub fn clear_cache(app: &AppHandle) -> std::io::Result<()> {
    if let Ok(dir) = app.path().app_cache_dir() {
        let icons = dir.join("icons");
        if icons.exists() {
            std::fs::remove_dir_all(&icons)?;
        }
    }
    Ok(())
}
