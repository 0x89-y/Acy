use crate::model::{ManagerStatus, Package, SearchHit, Source};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

pub mod choco;
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
}

pub fn for_source(source: Source) -> Box<dyn PackageSource> {
    match source {
        Source::Winget => Box::new(winget::Winget),
        Source::Scoop => Box::new(scoop::Scoop),
        Source::Choco => Box::new(choco::Choco),
        Source::Msstore => Box::new(msstore::Msstore),
    }
}

const INFO_TTL: Duration = Duration::from_secs(10 * 60);

type InfoCell = Arc<OnceCell<(Instant, Option<Package>)>>;

static INFO: LazyLock<Mutex<HashMap<(Source, String), InfoCell>>> = LazyLock::new(Default::default);

fn info_fresh(cell: &InfoCell) -> bool {
    cell.get().is_none_or(|(at, _)| at.elapsed() < INFO_TTL)
}

/// `info()` memoized for a few minutes, with concurrent calls for the same
/// package sharing one run. The app page and its icon both need `winget show`
/// for the same id at the same moment; this makes that one process, not two.
/// Errors aren't cached, so the next call retries.
pub async fn info_cached(source: Source, id: &str) -> anyhow::Result<Option<Package>> {
    let key = (source, id.to_lowercase());
    let cell = {
        let mut map = INFO.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(&key) {
            Some(cell) if info_fresh(cell) => cell.clone(),
            _ => {
                map.retain(|_, c| info_fresh(c));
                let cell = InfoCell::default();
                map.insert(key, cell.clone());
                cell
            }
        }
    };
    let (_, pkg) = cell
        .get_or_try_init(|| async { Ok::<_, anyhow::Error>((Instant::now(), for_source(source).info(id).await?)) })
        .await?;
    Ok(pkg.clone())
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
    hits.sort_by_cached_key(|p| p.name.to_lowercase());
    hits
}
