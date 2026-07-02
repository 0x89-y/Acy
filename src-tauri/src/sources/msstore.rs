use super::winget::{parse_show, parse_table};
use super::PackageSource;
use crate::model::{ManagerStatus, Package, Source};
use crate::runner;
use async_trait::async_trait;

pub struct Msstore;

const MAX_SEARCH: usize = 60;

fn s(v: &str) -> String {
    v.to_string()
}

fn map_rows(out: &str) -> Vec<Package> {
    parse_table(out)
        .into_iter()
        .filter_map(|row| {
            let name = row.get("Name").cloned().unwrap_or_default();
            let id = row.get("Id").cloned().filter(|v| !v.is_empty())?;
            let display = if name.is_empty() { id.clone() } else { name };
            let mut pkg = Package::new(id, display, Source::Msstore);
            pkg.version = row.get("Version").cloned().filter(|v| !v.is_empty());
            Some(pkg)
        })
        .collect()
}

#[async_trait]
impl PackageSource for Msstore {
    fn source(&self) -> Source {
        Source::Msstore
    }

    async fn status(&self) -> ManagerStatus {
        let available = runner::capture_ok("winget", &[s("--version")]).await.is_ok();
        ManagerStatus {
            source: Source::Msstore,
            available,
            needs_setup: false,
            detail: if available {
                None
            } else {
                Some("Microsoft Store search needs winget (ships with Windows 11)".into())
            },
        }
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Package>> {
        let out = runner::capture(
            "winget",
            &[
                s("search"),
                s("--query"),
                s(query),
                s("--source"),
                s("msstore"),
                s("--accept-source-agreements"),
            ],
        )
        .await?;
        let mut pkgs = map_rows(&out);
        pkgs.truncate(MAX_SEARCH);
        Ok(pkgs)
    }

    async fn list_installed(&self) -> anyhow::Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn list_updates(&self) -> anyhow::Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn info(&self, id: &str) -> anyhow::Result<Option<Package>> {
        let out = runner::capture(
            "winget",
            &[
                s("show"),
                s("--id"),
                s(id),
                s("-e"),
                s("--source"),
                s("msstore"),
                s("--accept-source-agreements"),
            ],
        )
        .await?;
        Ok(parse_show(&out, id).map(|mut p| {
            p.source = Source::Msstore;
            p
        }))
    }

    fn install_cmd(&self, id: &str) -> (String, Vec<String>) {
        (
            s("winget"),
            vec![
                s("install"),
                s("--id"),
                s(id),
                s("-e"),
                s("--source"),
                s("msstore"),
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
                s("--source"),
                s("msstore"),
                s("--accept-package-agreements"),
                s("--accept-source-agreements"),
            ],
        )
    }

    fn upgrade_all_cmd(&self) -> (String, Vec<String>) {
        (
            s("winget"),
            vec![
                s("upgrade"),
                s("--all"),
                s("--source"),
                s("msstore"),
                s("--accept-package-agreements"),
                s("--accept-source-agreements"),
            ],
        )
    }
}
