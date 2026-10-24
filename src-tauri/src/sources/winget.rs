use super::PackageSource;
use crate::model::{ManagerStatus, Package, Source};
use crate::runner::{self, ps_quote as quote};
use async_trait::async_trait;
use std::sync::atomic::{AtomicI8, Ordering};

pub struct Winget;

const MAX_SEARCH: usize = 60;

static MODULE_STATE: AtomicI8 = AtomicI8::new(0);

fn s(v: &str) -> String {
    v.to_string()
}

async fn module_available() -> bool {
    match MODULE_STATE.load(Ordering::Relaxed) {
        2 => return true,
        1 => return false,
        _ => {}
    }
    let found = runner::capture(
        "powershell",
        &runner::ps_args(
            "try { Import-Module Microsoft.WinGet.Client -ErrorAction Stop; \
             if (Get-Command Get-WinGetPackage -ErrorAction SilentlyContinue) { 'yes' } } catch { }",
        ),
    )
    .await
    .map(|o| o.contains("yes"))
    .unwrap_or(false);
    MODULE_STATE.store(if found { 2 } else { 1 }, Ordering::Relaxed);
    found
}

pub fn invalidate_module_cache() {
    MODULE_STATE.store(0, Ordering::Relaxed);
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
            ],
        )
        .await?;
        let mut pkgs = Self::map_cli_table(&out, false);
        pkgs.truncate(MAX_SEARCH);
        Ok(pkgs)
    }

    async fn list_installed(&self) -> anyhow::Result<Vec<Package>> {
        let mut pkgs = if module_available().await {
            let script = "Get-WinGetPackage | ForEach-Object { \
                 [pscustomobject]@{ Name=$_.Name; Id=$_.Id; Version=$_.InstalledVersion; \
                 Source=$_.Source } } | ConvertTo-Json -Depth 3";
            let out = runner::capture("powershell", &runner::ps_args(script)).await?;
            let rows = Self::map_module_rows(&out, true);
            if rows.is_empty() {
                cli_list_installed().await?
            } else {
                rows
            }
        } else {
            cli_list_installed().await?
        };
        if pkgs.iter().any(|p| p.id.starts_with("ARP\\")) {
            let map = tokio::task::spawn_blocking(crate::arp::scan)
                .await
                .unwrap_or_default();
            enrich_from_arp(&mut pkgs, &map);
        }
        Ok(pkgs)
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
                &[s("upgrade"), s("--accept-source-agreements")],
            )
            .await?;
            Ok(Self::map_cli_table(&out, true))
        }
    }

    async fn info(&self, id: &str) -> anyhow::Result<Option<Package>> {
        if id.starts_with("ARP\\") || id.starts_with("MSIX\\") {
            return Ok(None);
        }
        let out = runner::capture(
            "winget",
            &[
                s("show"),
                s("--id"),
                s(id),
                s("-e"),
                s("--accept-source-agreements"),
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
            ],
        )
    }

}

async fn cli_list_installed() -> anyhow::Result<Vec<Package>> {
    let out = runner::capture("winget", &[s("list"), s("--accept-source-agreements")]).await?;
    let pkgs = Winget::map_cli_table(&out, true);
    if pkgs.is_empty() {
        if out.trim().is_empty() {
            anyhow::bail!(
                "winget returned no output. It may be an older version; updating App Installer can help."
            );
        }
        anyhow::bail!(
            "Couldn't read winget's installed list (this can happen on non-English Windows). Install the WinGet PowerShell module in Settings \u{2192} Sources for reliable results."
        );
    }
    Ok(pkgs)
}

fn enrich_from_arp(
    pkgs: &mut [Package],
    map: &std::collections::HashMap<String, crate::arp::ArpInfo>,
) {
    if map.is_empty() {
        return;
    }
    for p in pkgs.iter_mut() {
        if !p.id.starts_with("ARP\\") {
            continue;
        }
        let subkey = p.id.rsplit('\\').next().unwrap_or_default();
        if let Some(info) = map.get(&crate::arp::norm(subkey)) {
            if info.publisher.is_some() {
                p.publisher = info.publisher.clone();
            }
            if info.install_location.is_some() {
                p.install_location = info.install_location.clone();
            }
        }
    }
}

fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
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

fn sanitize(out: &str) -> Vec<String> {
    out.split('\n')
        .map(|raw| {
            // winget ends lines with CRLF; drop that \r first, or the spinner
            // handling below (keep only what follows the last \r) would turn
            // every line into an empty string.
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let visible = raw.rsplit('\r').next().unwrap_or(raw);
            strip_ansi(visible).trim_end().to_string()
        })
        .collect()
}

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

#[derive(Clone, Copy)]
enum ShowField {
    Version,
    Publisher,
    PublisherUrl,
    Author,
    Description,
    Homepage,
    License,
    LicenseUrl,
    ReleaseNotes,
    ReleaseNotesUrl,
}

// `winget show` labels are translated into the Windows display language, so
// match every translation winget ships. Taken from winget-cli's
// Localization/Resources/*/winget.resw (ShowLabel* keys, trailing colon and
// padding removed): en, de, es, fr, it, ja, ko, pt-BR, ru, zh-CN, zh-TW.
const SHOW_LABELS: &[(ShowField, &[&str])] = &[
    (ShowField::Version, &["Version", "Versión", "Versione", "バージョン", "버전", "Versão", "Версия", "版本"]),
    (ShowField::Publisher, &["Publisher", "Herausgeber", "Editor", "Editore", "公開元", "게시자", "Fornecedor", "Издатель", "发布者", "發行者"]),
    (ShowField::PublisherUrl, &["Publisher Url", "Herausgeber-URL", "Dirección URL del editor", "ID de l’Editeur", "URL editore", "発行元 URL", "게시자 URL", "URL do Fornecedor", "URL-адрес издателя", "发布服务器 URL", "發行者 URL"]),
    (ShowField::Author, &["Author", "Autor", "Auteur", "Autore", "作成者", "만든 이", "Автор", "作者"]),
    (ShowField::Description, &["Description", "Beschreibung", "Descripción", "Descrizione", "説明", "설명", "Descrição", "Описание", "描述"]),
    (ShowField::Homepage, &["Homepage", "Startseite", "Página principal", "Page d’accueil", "Home page", "ホーム ページ", "홈페이지", "Página inicial", "Домашняя страница", "主页", "首頁"]),
    (ShowField::License, &["License", "Lizenz", "Licencia", "Licence", "Licenza", "ライセンス", "라이선스", "Licença", "Лицензия", "许可证", "授權"]),
    (ShowField::LicenseUrl, &["License Url", "Lizenz-URL", "Dirección URL de la licencia", "URL de la licence", "URL licenza", "ライセンス URL", "라이선스 URL", "URL da licença", "URL-адрес лицензии", "许可证 URL", "授權 URL"]),
    (ShowField::ReleaseNotes, &["Release Notes", "Versionshinweise", "Notas de la versión", "Notes de publication", "Note sulla versione", "リリース ノート", "릴리스 정보", "Notas de Versão", "Заметки о выпуске", "发行说明", "版本資訊"]),
    (ShowField::ReleaseNotesUrl, &["Release Notes Url", "URL der Versionshinweise", "Dirección URL de notas de la versión", "URL des notes de publication", "URL note sulla versione", "リリース ノート URL", "릴리스 정보 URL", "URL de Notas de Versão", "URL-адрес заметок о выпуске", "发行说明 URL", "版本資訊 Url"]),
];

const MAX_RELEASE_NOTES: usize = 4000;

fn show_field(label: &str) -> Option<ShowField> {
    let label = label.trim();
    SHOW_LABELS
        .iter()
        .find(|(_, names)| names.iter().any(|n| n.eq_ignore_ascii_case(label)))
        .map(|(f, _)| *f)
}

// "Label: value" - Chinese output uses the full-width colon.
fn split_label(line: &str) -> Option<(&str, &str)> {
    let at = line.find([':', '：'])?;
    let colon_len = line[at..].chars().next()?.len_utf8();
    Some((&line[..at], &line[at + colon_len..]))
}

fn apply_show_field(pkg: &mut Package, field: ShowField, value: String) {
    let d = &mut pkg.details;
    match field {
        ShowField::Version => pkg.version = Some(value),
        ShowField::Publisher => pkg.publisher = Some(value),
        ShowField::PublisherUrl => d.publisher_url = Some(value),
        ShowField::Author => d.author = Some(value),
        ShowField::Description => pkg.description = Some(value),
        ShowField::Homepage => pkg.homepage = Some(value),
        ShowField::License => d.license = Some(value),
        ShowField::LicenseUrl => d.license_url = Some(value),
        ShowField::ReleaseNotes => {
            d.release_notes = Some(if value.chars().count() > MAX_RELEASE_NOTES {
                let cut: String = value.chars().take(MAX_RELEASE_NOTES).collect();
                format!("{}…", cut.trim_end())
            } else {
                value
            })
        }
        ShowField::ReleaseNotesUrl => d.release_notes_url = Some(value),
    }
}

pub(super) fn parse_show(out: &str, id: &str) -> Option<Package> {
    let mut pkg = Package::new(id, id, Source::Winget);
    let mut found = false;
    // A field whose value may continue on the following indented lines
    // (Description, Release Notes); anything under an unknown label is skipped.
    let mut current: Option<(ShowField, Vec<String>)> = None;
    let id_suffix = format!("[{id}]");

    let mut flush = |pkg: &mut Package, current: &mut Option<(ShowField, Vec<String>)>| {
        if let Some((field, lines)) = current.take() {
            let value = lines.join("\n").trim().to_string();
            if !value.is_empty() {
                apply_show_field(pkg, field, value);
                found = true;
            }
        }
    };

    for line in sanitize(out) {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            if let Some((_, lines)) = current.as_mut() {
                lines.push(line.trim().to_string());
            }
            continue;
        }
        flush(&mut pkg, &mut current);
        let trimmed = line.trim();
        // "Found <name> [<id>]" in whatever language: one word, then the name.
        let head_end = trimmed.len().saturating_sub(id_suffix.len());
        if trimmed.is_char_boundary(head_end) && trimmed[head_end..].eq_ignore_ascii_case(&id_suffix) {
            if let Some((_, name)) = trimmed[..head_end].trim().split_once(char::is_whitespace) {
                pkg.name = name.trim().to_string();
            }
            continue;
        }
        if let Some((label, value)) = split_label(trimmed) {
            if let Some(field) = show_field(label) {
                current = Some((field, vec![value.trim().to_string()]));
            }
        }
    }
    flush(&mut pkg, &mut current);

    found.then_some(pkg)
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
    fn parses_crlf_table() {
        let out = "Name    Id         Version  Source\r\n\
                   ----------------------------------\r\n\
                   Git     Git.Git    2.45.0   winget\r\n";
        let pkgs = Winget::map_cli_table(out, false);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].id, "Git.Git");
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

    #[test]
    fn parses_german_show_output() {
        // Real `winget show` output on German Windows (trimmed), CRLF and all,
        // with the spinner winget prints before it.
        let out = [
            "\r-\r\\\r|\r \rGefunden Alacritty [Alacritty.Alacritty]",
            "Version: 0.17.0",
            "Herausgeber: Alacritty",
            "Herausgeber-URL: https://github.com/alacritty/alacritty",
            "Autor: Alacritty",
            "Moniker: alacritty",
            "Beschreibung: Alacritty is a free and open-source GPU accelerated terminal emulator.",
            "Startseite: https://github.com/alacritty/alacritty",
            "Lizenz: Apache-2.0",
            "Lizenz-URL: https://github.com/alacritty/alacritty/blob/HEAD/LICENSE-APACHE",
            "Versionshinweise:",
            "  Packaging",
            "  - Fixed invalid logo SVG attributes",
            "URL der Versionshinweise: https://github.com/alacritty/alacritty/releases/tag/v0.17.0",
            "Dokumentation:",
            "  Wiki: https://github.com/alacritty/alacritty/wiki",
            "Installationsprogramm:",
            "  Installertyp: wix",
        ]
        .join("\r\n");
        let pkg = parse_show(&out, "Alacritty.Alacritty").unwrap();
        let d = &pkg.details;
        assert_eq!(pkg.name, "Alacritty");
        assert_eq!(pkg.version.as_deref(), Some("0.17.0"));
        assert_eq!(pkg.publisher.as_deref(), Some("Alacritty"));
        assert_eq!(pkg.homepage.as_deref(), Some("https://github.com/alacritty/alacritty"));
        assert!(pkg.description.as_deref().unwrap().starts_with("Alacritty is a free"));
        assert_eq!(d.author.as_deref(), Some("Alacritty"));
        assert_eq!(d.license.as_deref(), Some("Apache-2.0"));
        assert!(d.license_url.as_deref().unwrap().ends_with("LICENSE-APACHE"));
        assert_eq!(
            d.release_notes.as_deref(),
            Some("Packaging\n- Fixed invalid logo SVG attributes")
        );
        assert!(d.release_notes_url.as_deref().unwrap().ends_with("v0.17.0"));
        assert_eq!(d.publisher_url.as_deref(), Some("https://github.com/alacritty/alacritty"));
    }

    #[test]
    fn parses_full_width_colon_and_padded_labels() {
        let out = "已找到 Git [Git.Git]\n版本： 2.45.0\n许可证： GPL-2.0\nVersion : 2.45.0\n";
        let pkg = parse_show(out, "Git.Git").unwrap();
        assert_eq!(pkg.name, "Git");
        assert_eq!(pkg.version.as_deref(), Some("2.45.0"));
        assert_eq!(pkg.details.license.as_deref(), Some("GPL-2.0"));
    }

    #[test]
    fn show_without_known_fields_is_none() {
        let out = "Es wurde kein Paket gefunden, das den Eingabekriterien entspricht.\n";
        assert!(parse_show(out, "Nope.Nope").is_none());
    }
}
