use super::PackageSource;
use crate::model::{ManagerStatus, Package, Source};
use crate::runner;
use async_trait::async_trait;
use tokio::sync::OnceCell;

/// winget backend. Prefers the official `Microsoft.WinGet.Client` PowerShell
/// module for read operations (clean JSON), and falls back to parsing the
/// `winget` CLI tables when the module is not installed. Write operations
/// always stream the CLI so the UI can show live progress.
pub struct Winget;

const MAX_SEARCH: usize = 60;

static MODULE_AVAILABLE: OnceCell<bool> = OnceCell::const_new();

fn s(v: &str) -> String {
    v.to_string()
}

/// Single-quote a value for safe embedding in a PowerShell command.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

async fn module_available() -> bool {
    *MODULE_AVAILABLE
        .get_or_init(|| async {
            runner::capture(
                "powershell",
                &runner::ps_args(
                    "if (Get-Module -ListAvailable -Name Microsoft.WinGet.Client) { 'yes' }",
                ),
            )
            .await
            .map(|o| o.contains("yes"))
            .unwrap_or(false)
        })
        .await
}

fn json_rows(text: &str) -> Vec<serde_json::Value> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::Array(a)) => a,
        Ok(serde_json::Value::Null) => Vec::new(),
        Ok(v) => vec![v],
        Err(_) => Vec::new(),
    }
}

fn field(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

impl Winget {
    // ---- PowerShell module (preferred) ----

    fn map_module_rows(out: &str, mark_installed: bool) -> Vec<Package> {
        json_rows(out)
            .into_iter()
            .filter_map(|v| {
                let name = field(&v, "Name").unwrap_or_default();
                let id = field(&v, "Id").unwrap_or_else(|| name.clone());
                if id.is_empty() {
                    return None;
                }
                let display = if name.is_empty() { id.clone() } else { name };
                let mut pkg = Package::new(id, display, Source::Winget);
                pkg.version = field(&v, "Version");
                pkg.available_version = field(&v, "Available");
                pkg.publisher = field(&v, "Source");
                pkg.installed = mark_installed;
                Some(pkg)
            })
            .collect()
    }

    // ---- CLI table fallback ----

    fn map_cli_table(out: &str, mark_installed: bool) -> Vec<Package> {
        parse_table(out)
            .into_iter()
            .filter_map(|row| {
                let name = row.get("Name").cloned().unwrap_or_default();
                let id = row.get("Id").cloned().filter(|v| !v.is_empty())?;
                let display = if name.is_empty() { id.clone() } else { name };
                let mut pkg = Package::new(id, display, Source::Winget);
                pkg.version = row.get("Version").cloned().filter(|v| !v.is_empty());
                pkg.available_version = row.get("Available").cloned().filter(|v| !v.is_empty());
                pkg.installed = mark_installed;
                Some(pkg)
            })
            .collect()
    }
}

#[async_trait]
impl PackageSource for Winget {
    fn source(&self) -> Source {
        Source::Winget
    }

    async fn status(&self) -> ManagerStatus {
        let available = runner::capture_ok("winget", &[s("--version")]).await.is_ok();
        let has_module = available && module_available().await;
        ManagerStatus {
            source: Source::Winget,
            available,
            needs_setup: available && !has_module,
            detail: if !available {
                Some("winget is not available".into())
            } else if !has_module {
                Some("Optional: install the WinGet PowerShell module for faster results".into())
            } else {
                None
            },
        }
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Package>> {
        // Preferred: the PowerShell module (clean JSON). If it yields nothing
        // (module missing or failed to load), fall back to parsing the CLI.
        if module_available().await {
            let script = format!(
                "Find-WinGetPackage -Query {} -Source winget | ForEach-Object {{ \
                 [pscustomobject]@{{ Name=$_.Name; Id=$_.Id; Version=$_.Version }} }} | \
                 ConvertTo-Json -Depth 3",
                quote(query)
            );
            let out = runner::capture("powershell", &runner::ps_args(&script))
                .await
                .unwrap_or_default();
            let mut pkgs = Self::map_module_rows(&out, false);
            if !pkgs.is_empty() {
                pkgs.truncate(MAX_SEARCH);
                return Ok(pkgs);
            }
        }

        let out = runner::capture(
            "winget",
            &[
                s("search"),
                s("--query"),
                s(query),
                s("--source"),
                s("winget"),
                s("--accept-source-agreements"),
                s("--disable-interactivity"),
            ],
        )
        .await?;
        let mut pkgs = Self::map_cli_table(&out, false);
        pkgs.truncate(MAX_SEARCH);
        Ok(pkgs)
    }

    async fn list_installed(&self) -> anyhow::Result<Vec<Package>> {
        if module_available().await {
            let script = "Get-WinGetPackage | ForEach-Object { \
                 [pscustomobject]@{ Name=$_.Name; Id=$_.Id; Version=$_.InstalledVersion; \
                 Source=$_.Source } } | ConvertTo-Json -Depth 3";
            let out = runner::capture("powershell", &runner::ps_args(script)).await?;
            Ok(Self::map_module_rows(&out, true))
        } else {
            let out = runner::capture(
                "winget",
                &[s("list"), s("--accept-source-agreements"), s("--disable-interactivity")],
            )
            .await?;
            Ok(Self::map_cli_table(&out, true))
        }
    }

    async fn list_updates(&self) -> anyhow::Result<Vec<Package>> {
        if module_available().await {
            let script = "Get-WinGetPackage | Where-Object IsUpdateAvailable | ForEach-Object { \
                 [pscustomobject]@{ Name=$_.Name; Id=$_.Id; Version=$_.InstalledVersion; \
                 Available=(@($_.AvailableVersions)[0]) } } | ConvertTo-Json -Depth 3";
            let out = runner::capture("powershell", &runner::ps_args(script)).await?;
            Ok(Self::map_module_rows(&out, true))
        } else {
            let out = runner::capture(
                "winget",
                &[s("upgrade"), s("--accept-source-agreements"), s("--disable-interactivity")],
            )
            .await?;
            Ok(Self::map_cli_table(&out, true))
        }
    }

    async fn info(&self, id: &str) -> anyhow::Result<Option<Package>> {
        // `winget show` works without the module and carries description/homepage.
        let out = runner::capture(
            "winget",
            &[
                s("show"),
                s("--id"),
                s(id),
                s("-e"),
                s("--accept-source-agreements"),
                s("--disable-interactivity"),
            ],
        )
        .await?;
        Ok(parse_show(&out, id))
    }

    fn install_cmd(&self, id: &str) -> (String, Vec<String>) {
        (
            s("winget"),
            vec![
                s("install"),
                s("--id"),
                s(id),
                s("-e"),
                s("--silent"),
                s("--accept-package-agreements"),
                s("--accept-source-agreements"),
                s("--disable-interactivity"),
            ],
        )
    }

    fn uninstall_cmd(&self, id: &str) -> (String, Vec<String>) {
        (
            s("winget"),
            vec![
                s("uninstall"),
                s("--id"),
                s(id),
                s("-e"),
                s("--silent"),
                s("--accept-source-agreements"),
                s("--disable-interactivity"),
            ],
        )
    }

    fn upgrade_cmd(&self, id: &str) -> (String, Vec<String>) {
        (
            s("winget"),
            vec![
                s("upgrade"),
                s("--id"),
                s(id),
                s("-e"),
                s("--silent"),
                s("--accept-package-agreements"),
                s("--accept-source-agreements"),
                s("--disable-interactivity"),
            ],
        )
    }

    fn upgrade_all_cmd(&self) -> (String, Vec<String>) {
        (
            s("winget"),
            vec![
                s("upgrade"),
                s("--all"),
                s("--silent"),
                s("--accept-package-agreements"),
                s("--accept-source-agreements"),
                s("--disable-interactivity"),
            ],
        )
    }
}

/// Strip ANSI escape sequences from a line.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip until the terminating letter of the escape sequence.
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Collapse winget's `\r` progress redraws and strip ANSI, one line per row.
fn sanitize(out: &str) -> Vec<String> {
    out.split('\n')
        .map(|raw| {
            let visible = raw.rsplit('\r').next().unwrap_or(raw);
            strip_ansi(visible).trim_end().to_string()
        })
        .collect()
}

/// Parse a winget CLI table (search / list / upgrade) into rows keyed by a
/// normalized header: "Name" / "Id" / "Version" / "Available".
///
/// winget localizes some headers (e.g. "Match"), capitalizes the id column as
/// "ID", and separates narrow columns with a single space. So rather than rely
/// on fixed labels and spacing, we locate the `----` separator line, take the
/// header above it, find columns by case-insensitive label, and read the
/// single-token fields (Id / Version / Available) as the first whitespace token
/// at the column offset.
pub(super) fn parse_table(out: &str) -> Vec<std::collections::HashMap<String, String>> {
    let lines = sanitize(out);

    let sep = lines.iter().position(|l| {
        let t = l.trim();
        let dashes = t.chars().filter(|c| *c == '-').count();
        t.len() >= 3 && dashes * 5 >= t.len() * 4
    });
    let Some(sep) = sep else { return Vec::new() };
    if sep == 0 {
        return Vec::new();
    }

    let header = lines[sep - 1].to_lowercase();
    let offset = |label: &str| header.find(label).map(|b| header[..b].chars().count());

    let (Some(id_off), Some(ver_off)) = (offset("id"), offset("version")) else {
        return Vec::new();
    };
    let name_off = offset("name").unwrap_or(0);
    let avail_off = offset("available");
    let source_off = offset("source");

    // First whitespace token within [start, end).
    let first_token = |chars: &[char], start: usize, end: usize| -> String {
        let end = end.min(chars.len());
        let mut i = start.min(end);
        while i < end && chars[i] == ' ' {
            i += 1;
        }
        let begin = i;
        while i < end && chars[i] != ' ' {
            i += 1;
        }
        chars[begin..i].iter().collect()
    };
    let region = |chars: &[char], a: usize, b: usize| -> String {
        let a = a.min(chars.len());
        let b = b.min(chars.len());
        if a >= b {
            return String::new();
        }
        chars[a..b].iter().collect::<String>().trim().to_string()
    };

    let mut rows = Vec::new();
    for line in lines.iter().skip(sep + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();

        let id = first_token(&chars, id_off, ver_off);
        if id.is_empty() {
            continue;
        }
        let mut row = std::collections::HashMap::new();
        row.insert("Name".to_string(), region(&chars, name_off, id_off));
        row.insert("Id".to_string(), id);
        let ver_end = avail_off.or(source_off).unwrap_or(len);
        row.insert("Version".to_string(), first_token(&chars, ver_off, ver_end));
        if let Some(ao) = avail_off {
            let avail = first_token(&chars, ao, source_off.unwrap_or(len));
            if !avail.is_empty() {
                row.insert("Available".to_string(), avail);
            }
        }
        rows.push(row);
    }
    rows
}

/// Parse `winget show` key/value output into a single package.
pub(super) fn parse_show(out: &str, id: &str) -> Option<Package> {
    let mut pkg = Package::new(id, id, Source::Winget);
    let mut found = false;
    for line in sanitize(out) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Found ") {
            // "Found Mozilla Firefox [Mozilla.Firefox]"
            if let Some(end) = rest.rfind(" [") {
                pkg.name = rest[..end].trim().to_string();
                found = true;
            }
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let value = value.trim().to_string();
            if value.is_empty() {
                continue;
            }
            match key.trim() {
                "Version" => pkg.version = Some(value),
                "Publisher" => pkg.publisher = Some(value),
                "Homepage" => pkg.homepage = Some(value),
                "Description" => pkg.description = Some(value),
                _ => {}
            }
            found = true;
        }
    }
    if found {
        Some(pkg)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_table_by_offsets() {
        let out = "\
Name               Id                  Version   Source
-----------------------------------------------------------
Mozilla Firefox    Mozilla.Firefox     126.0     winget
Brave              Brave.Brave         1.66.110  winget
";
        let pkgs = Winget::map_cli_table(out, false);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].id, "Mozilla.Firefox");
        assert_eq!(pkgs[0].name, "Mozilla Firefox");
        assert_eq!(pkgs[0].version.as_deref(), Some("126.0"));
    }

    #[test]
    fn parses_localized_caps_id_header() {
        // Real-world winget: "ID" is capitalized, the trailing column is
        // localized ("Übereinstimmung"), and Version is one space from it.
        let nw = 25usize;
        let iw = 21usize;
        let header = format!(
            "{:<nw$}{:<iw$}{} {}",
            "Name", "ID", "Version", "Übereinstimmung"
        );
        let sep = "-".repeat(70);
        let row = format!(
            "{:<nw$}{:<iw$}{} {}",
            "Mozilla Firefox (en-US)", "Mozilla.Firefox", "152.0.2", "Moniker: firefox"
        );
        let out = format!("{header}\n{sep}\n{row}\n");
        let pkgs = Winget::map_cli_table(&out, false);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].id, "Mozilla.Firefox");
        assert_eq!(pkgs[0].version.as_deref(), Some("152.0.2"));
        assert!(pkgs[0].name.starts_with("Mozilla Firefox"));
    }

    #[test]
    fn parses_upgrade_table_with_available() {
        let out = "\
Name        Id              Version   Available  Source
----------------------------------------------------------
Git         Git.Git         2.44.0    2.45.0     winget
";
        let pkgs = Winget::map_cli_table(out, true);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].version.as_deref(), Some("2.44.0"));
        assert_eq!(pkgs[0].available_version.as_deref(), Some("2.45.0"));
        assert!(pkgs[0].installed);
    }

    #[test]
    fn maps_module_json() {
        let json = r#"[{"Name":"Mozilla Firefox","Id":"Mozilla.Firefox","Version":"126.0"}]"#;
        let pkgs = Winget::map_module_rows(json, false);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].id, "Mozilla.Firefox");
    }

    #[test]
    fn parses_show_output() {
        let out = "Found Mozilla Firefox [Mozilla.Firefox]\nVersion: 126.0\nPublisher: Mozilla\nHomepage: https://www.mozilla.org\nDescription: A web browser\n";
        let pkg = parse_show(out, "Mozilla.Firefox").unwrap();
        assert_eq!(pkg.name, "Mozilla Firefox");
        assert_eq!(pkg.homepage.as_deref(), Some("https://www.mozilla.org"));
    }
}
