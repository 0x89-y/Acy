use crate::model::{ManagerStatus, Package, SearchHit, Source};
use async_trait::async_trait;
use std::collections::BTreeMap;

pub mod choco;
pub mod local;
pub mod msstore;
pub mod scoop;
pub mod winget;

#[async_trait]
pub trait PackageSource: Send + Sync {
    #[allow(dead_code)]
    fn source(&self) -> Source;

    async fn status(&self) -> ManagerStatus;

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Package>>;
    async fn list_installed(&self) -> anyhow::Result<Vec<Package>>;
    async fn list_updates(&self) -> anyhow::Result<Vec<Package>>;
    async fn info(&self, id: &str) -> anyhow::Result<Option<Package>>;

    fn install_cmd(&self, id: &str) -> (String, Vec<String>);
    fn uninstall_cmd(&self, id: &str) -> (String, Vec<String>);
    fn upgrade_cmd(&self, id: &str) -> (String, Vec<String>);
    fn upgrade_all_cmd(&self) -> (String, Vec<String>);
}

pub fn for_source(source: Source) -> Box<dyn PackageSource> {
    match source {
        Source::Winget => Box::new(winget::Winget),
        Source::Scoop => Box::new(scoop::Scoop),
        Source::Choco => Box::new(choco::Choco),
        Source::Msstore => Box::new(msstore::Msstore),
        Source::Local => Box::new(local::Local),
    }
}

pub fn all() -> Vec<Box<dyn PackageSource>> {
    vec![
        Box::new(winget::Winget),
        Box::new(scoop::Scoop),
        Box::new(choco::Choco),
        Box::new(msstore::Msstore),
    ]
}

fn dedup_key(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub fn merge(packages: Vec<Package>) -> Vec<SearchHit> {
    let mut map: BTreeMap<String, SearchHit> = BTreeMap::new();

    for pkg in packages {
        let key = dedup_key(&pkg.name);
        let hit = map.entry(key).or_insert_with(|| SearchHit {
            name: pkg.name.clone(),
            publisher: pkg.publisher.clone(),
            description: pkg.description.clone(),
            installed: false,
            variants: Vec::new(),
        });
        hit.installed |= pkg.installed;
        if hit.publisher.is_none() {
            hit.publisher = pkg.publisher.clone();
        }
        if hit.description.is_none() {
            hit.description = pkg.description.clone();
        }
        hit.variants.push(pkg);
    }

    let mut hits: Vec<SearchHit> = map.into_values().collect();
    for hit in &mut hits {
        hit.variants.sort_by_key(|v| v.source.priority());
    }
    hits.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    hits
}
