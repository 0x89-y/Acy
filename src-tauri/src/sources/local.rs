use super::PackageSource;
use crate::model::{ManagerStatus, Package, Source};
use async_trait::async_trait;

/// "Install from a file" source. The `id` is a path to a local or network
/// installer (.exe / .msi). It only installs — there is nothing to search,
/// list, or update — so the read methods return empty.
pub struct Local;

fn s(v: &str) -> String {
    v.to_string()
}

fn is_msi(path: &str) -> bool {
    path.to_lowercase().trim_end().ends_with(".msi")
}

#[async_trait]
impl PackageSource for Local {
    fn source(&self) -> Source {
        Source::Local
    }

    async fn status(&self) -> ManagerStatus {
        // Always usable: it just runs a file the user chose.
        ManagerStatus {
            source: Source::Local,
            available: true,
            needs_setup: false,
            detail: None,
        }
    }

    async fn search(&self, _query: &str) -> anyhow::Result<Vec<Package>> {
        Ok(Vec::new())
    }
    async fn list_installed(&self) -> anyhow::Result<Vec<Package>> {
        Ok(Vec::new())
    }
    async fn list_updates(&self) -> anyhow::Result<Vec<Package>> {
        Ok(Vec::new())
    }
    async fn info(&self, _id: &str) -> anyhow::Result<Option<Package>> {
        Ok(None)
    }

    /// `id` is the installer path. MSIs go through msiexec; EXEs are run directly
    /// so their own installer UI appears.
    fn install_cmd(&self, id: &str) -> (String, Vec<String>) {
        if is_msi(id) {
            (s("msiexec"), vec![s("/i"), s(id)])
        } else {
            (id.to_string(), Vec::new())
        }
    }

    fn uninstall_cmd(&self, id: &str) -> (String, Vec<String>) {
        if is_msi(id) {
            (s("msiexec"), vec![s("/x"), s(id)])
        } else {
            (id.to_string(), Vec::new())
        }
    }

    fn upgrade_cmd(&self, id: &str) -> (String, Vec<String>) {
        self.install_cmd(id)
    }

    fn upgrade_all_cmd(&self) -> (String, Vec<String>) {
        // Never called for a local source; there is nothing to upgrade.
        (s("cmd"), vec![s("/c"), s("echo"), s("nothing to upgrade")])
    }
}
