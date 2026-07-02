use super::PackageSource;
use crate::model::{ManagerStatus, Package, Source};
use crate::runner;
use async_trait::async_trait;
use serde::Deserialize;

pub struct Scoop;

const MAX_SEARCH: usize = 60;

fn s(v: &str) -> String {
    v.to_string()
}

#[derive(Deserialize)]
struct ExportApp {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Source")]
    source: Option<String>,
    #[serde(rename = "Version")]
    version: Option<String>,
}

#[derive(Deserialize)]
struct ExportFile {
    apps: Vec<ExportApp>,
}

fn table_rows(out: &str) -> Vec<Vec<String>> {
    let lines: Vec<&str> = out.lines().collect();
    let sep = lines.iter().position(|l| l.trim_start().starts_with("----"));
    let Some(sep) = sep else { return Vec::new() };
    lines[sep + 1..]
        .iter()
        .map(|l| l.trim())
        .take_while(|l| !l.is_empty())
        .map(|l| l.split_whitespace().map(|c| c.to_string()).collect())
        .filter(|cols: &Vec<String>| !cols.is_empty())
        .collect()
}

impl Scoop {
    fn parse_search(out: &str) -> Vec<Package> {
        table_rows(out)
            .into_iter()
            .filter_map(|cols| {
                let name = cols.first()?.clone();
                let mut pkg = Package::new(name.clone(), name, Source::Scoop);
                pkg.version = cols.get(1).cloned();
                pkg.publisher = cols.get(2).cloned();
                Some(pkg)
            })
            .collect()
    }
}

#[async_trait]
impl PackageSource for Scoop {
    fn source(&self) -> Source {
        Source::Scoop
    }

    async fn status(&self) -> ManagerStatus {
        let out = runner::capture(
            "powershell",
            &runner::ps_args("if (Get-Command scoop -ErrorAction SilentlyContinue) { 'yes' }"),
        )
        .await
        .unwrap_or_default();
        let available = out.contains("yes");
        ManagerStatus {
            source: Source::Scoop,
            available,
            needs_setup: false,
            detail: if available {
                None
            } else {
                Some("Scoop is not installed".into())
            },
        }
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Package>> {
        let script = format!("scoop search {}", quote(query));
        let out = runner::capture("powershell", &runner::ps_args(&script)).await?;
        let mut pkgs = Self::parse_search(&out);
        pkgs.truncate(MAX_SEARCH);
        Ok(pkgs)
    }

    async fn list_installed(&self) -> anyhow::Result<Vec<Package>> {
        let out = runner::capture("powershell", &runner::ps_args("scoop export")).await?;
        let parsed: ExportFile = serde_json::from_str(out.trim())
            .map_err(|e| anyhow::anyhow!("could not parse `scoop export`: {e}"))?;
        let pkgs = parsed
            .apps
            .into_iter()
            .map(|a| {
                let mut pkg = Package::new(a.name.clone(), a.name, Source::Scoop);
                pkg.version = a.version;
                pkg.publisher = a.source;
                pkg.installed = true;
                pkg
            })
            .collect();
        Ok(pkgs)
    }

    async fn list_updates(&self) -> anyhow::Result<Vec<Package>> {
        let out = runner::capture("powershell", &runner::ps_args("scoop status")).await?;
        let pkgs = table_rows(&out)
            .into_iter()
            .filter_map(|cols| {
                let name = cols.first()?.clone();
                let installed = cols.get(1).cloned();
                let latest = cols.get(2).cloned();
                if latest.is_none() || latest == installed {
                    return None;
                }
                let mut pkg = Package::new(name.clone(), name, Source::Scoop);
                pkg.version = installed;
                pkg.available_version = latest;
                pkg.installed = true;
                Some(pkg)
            })
            .collect();
        Ok(pkgs)
    }

    async fn info(&self, id: &str) -> anyhow::Result<Option<Package>> {
        let script = format!("scoop info {}", quote(id));
        let out = runner::capture("powershell", &runner::ps_args(&script)).await?;
        let mut pkg = Package::new(id, id, Source::Scoop);
        let mut found = false;
        for line in out.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();
                if value.is_empty() {
                    continue;
                }
                found = true;
                match key.as_str() {
                    "name" => pkg.name = value,
                    "description" => pkg.description = Some(value),
                    "version" => pkg.version = Some(value),
                    "website" => pkg.homepage = Some(value),
                    "bucket" => pkg.publisher = Some(value),
                    _ => {}
                }
            }
        }
        Ok(if found { Some(pkg) } else { None })
    }

    fn install_cmd(&self, id: &str) -> (String, Vec<String>) {
        (s("powershell"), runner::ps_args(&format!("scoop install {}", quote(id))))
    }

    fn uninstall_cmd(&self, id: &str) -> (String, Vec<String>) {
        (s("powershell"), runner::ps_args(&format!("scoop uninstall {}", quote(id))))
    }

    fn upgrade_cmd(&self, id: &str) -> (String, Vec<String>) {
        (s("powershell"), runner::ps_args(&format!("scoop update {}", quote(id))))
    }

    fn upgrade_all_cmd(&self) -> (String, Vec<String>) {
        (s("powershell"), runner::ps_args("scoop update *"))
    }
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub fn needed_bucket(id: &str) -> Option<String> {
    let buckets = scoop_home()?.join("buckets");
    if let Some((bucket, _)) = id.split_once('/') {
        (!buckets.join(bucket).is_dir()).then(|| bucket.to_string())
    } else if manifest_in_added_bucket(&buckets, id) {
        None
    } else {
        Some("extras".to_string())
    }
}

fn scoop_home() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    match std::env::var("SCOOP") {
        Ok(s) if !s.is_empty() => Some(PathBuf::from(s)),
        _ => std::env::var("USERPROFILE").ok().map(|u| PathBuf::from(u).join("scoop")),
    }
}

fn manifest_in_added_bucket(buckets: &std::path::Path, id: &str) -> bool {
    let file = format!("{id}.json");
    let Ok(entries) = std::fs::read_dir(buckets) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let dir = entry.path();
        dir.is_dir() && (dir.join("bucket").join(&file).is_file() || dir.join(&file).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_table() {
        let out = "\nName  Version  Source  Binaries\n----  -------  ------  --------\nvlc   3.0.20   extras\ngit   2.45.0   main\n";
        let pkgs = Scoop::parse_search(out);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].id, "vlc");
        assert_eq!(pkgs[0].version.as_deref(), Some("3.0.20"));
        assert_eq!(pkgs[0].publisher.as_deref(), Some("extras"));
    }

    #[test]
    fn parses_export_json() {
        let json = r#"{"buckets":[],"apps":[{"Name":"7zip","Source":"main","Version":"23.01"}]}"#;
        let parsed: ExportFile = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.apps.len(), 1);
        assert_eq!(parsed.apps[0].name, "7zip");
    }
}
