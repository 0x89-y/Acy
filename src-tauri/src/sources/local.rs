use super::PackageSource;
use crate::model::{ManagerStatus, Package, Source};
use async_trait::async_trait;

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
}
