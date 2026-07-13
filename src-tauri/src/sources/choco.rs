use super::PackageSource;
use crate::model::{ManagerStatus, Package, Source};
use crate::runner;
use async_trait::async_trait;

pub struct Choco;

const MAX_SEARCH: usize = 60;

fn s(v: &str) -> String {
    v.to_string()
}

impl Choco {
    fn parse_rows(out: &str, source: Source) -> Vec<Package> {
        out.lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || !line.contains('|') {
                    return None;
                }
                let mut parts = line.split('|');
                let id = parts.next()?.trim();
                let version = parts.next().map(|v| v.trim().to_string());
                if id.is_empty() {
                    return None;
                }
                let mut pkg = Package::new(id, id, source);
                pkg.version = version.filter(|v| !v.is_empty());
                Some(pkg)
            })
            .collect()
    }
}

#[async_trait]
impl PackageSource for Choco {
    fn source(&self) -> Source {
        Source::Choco
    }

    async fn status(&self) -> ManagerStatus {
        let available = runner::capture_ok("choco", &[s("--version")]).await.is_ok();
        ManagerStatus {
            source: Source::Choco,
            available,
            needs_setup: false,
            detail: if available {
                None
            } else {
                Some("Chocolatey is not installed".into())
            },
        }
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Package>> {
        let out = runner::capture("choco", &[s("search"), s(query), s("-r")]).await?;
        let mut pkgs = Self::parse_rows(&out, Source::Choco);
        pkgs.truncate(MAX_SEARCH);
        Ok(pkgs)
    }

    async fn list_installed(&self) -> anyhow::Result<Vec<Package>> {
        let out = runner::capture("choco", &[s("list"), s("-r")]).await?;
        let mut pkgs = Self::parse_rows(&out, Source::Choco);
        for p in &mut pkgs {
            p.installed = true;
        }
        Ok(pkgs)
    }

    async fn list_updates(&self) -> anyhow::Result<Vec<Package>> {
        let out = runner::capture("choco", &[s("outdated"), s("-r")]).await?;
        let pkgs = out
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || !line.contains('|') {
                    return None;
                }
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() < 3 || parts[0].trim().is_empty() {
                    return None;
                }
                let id = parts[0].trim();
                let mut pkg = Package::new(id, id, Source::Choco);
                pkg.version = Some(parts[1].trim().to_string());
                pkg.available_version = Some(parts[2].trim().to_string());
                pkg.installed = true;
                Some(pkg)
            })
            .collect();
        Ok(pkgs)
    }

    async fn info(&self, id: &str) -> anyhow::Result<Option<Package>> {
        let out = runner::capture("choco", &[s("search"), s(id), s("--exact"), s("-r")]).await?;
        Ok(Self::parse_rows(&out, Source::Choco).into_iter().next())
    }

    fn install_cmd(&self, id: &str) -> (String, Vec<String>) {
        (s("choco"), vec![s("install"), s(id), s("-y"), s("--no-progress")])
    }

    fn uninstall_cmd(&self, id: &str) -> (String, Vec<String>) {
        (s("choco"), vec![s("uninstall"), s(id), s("-y"), s("--no-progress")])
    }

    fn upgrade_cmd(&self, id: &str) -> (String, Vec<String>) {
        (s("choco"), vec![s("upgrade"), s(id), s("-y"), s("--no-progress")])
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pipe_rows() {
        let out = "vlc|3.0.20\r\ngit|2.45.0\r\n";
        let pkgs = Choco::parse_rows(out, Source::Choco);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].id, "vlc");
        assert_eq!(pkgs[0].version.as_deref(), Some("3.0.20"));
        assert_eq!(pkgs[1].id, "git");
    }

    #[test]
    fn skips_noise_lines() {
        let out = "Chocolatey v2.2.2\nvlc|3.0.20\n3 packages found.\n";
        let pkgs = Choco::parse_rows(out, Source::Choco);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].id, "vlc");
    }
}
