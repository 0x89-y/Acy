use serde::{Deserialize, Serialize};

/// Which package manager a package comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Winget,
    Scoop,
    Choco,
    /// Microsoft Store apps, reached through winget's `msstore` source.
    Msstore,
    /// A local/network installer file the user points at (.exe / .msi). Not a
    /// package manager: no search, list, or updates.
    Local,
}

impl Source {
    /// Lower number = preferred when the same app exists in several managers.
    pub fn priority(&self) -> u8 {
        match self {
            Source::Winget => 0,
            Source::Scoop => 1,
            Source::Choco => 2,
            Source::Msstore => 3,
            Source::Local => 4,
        }
    }
}

/// A normalized package, regardless of which manager produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    /// Source-specific identifier used for install/uninstall
    /// (winget PackageIdentifier, scoop `bucket/name`, choco id).
    pub id: String,
    pub name: String,
    pub source: Source,
    pub version: Option<String>,
    /// Set when an update is available (installed < available).
    pub available_version: Option<String>,
    pub publisher: Option<String>,
    /// Filesystem install path (from the ARP registry), used to categorize
    /// launcher-installed apps like games. Only populated for winget ARP entries.
    pub install_location: Option<String>,
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub installed: bool,
}

impl Package {
    pub fn new(id: impl Into<String>, name: impl Into<String>, source: Source) -> Self {
        Package {
            id: id.into(),
            name: name.into(),
            source,
            version: None,
            available_version: None,
            publisher: None,
            install_location: None,
            homepage: None,
            description: None,
            installed: false,
        }
    }
}

/// One search result, grouping the same app across managers into a single card.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub name: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub installed: bool,
    /// One entry per manager that offers this app, in preference order.
    pub variants: Vec<Package>,
}

/// Availability of a manager on this machine, sent to the UI at startup.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerStatus {
    pub source: Source,
    pub available: bool,
    /// True when the manager is present but needs extra setup
    /// (e.g. winget without the PowerShell module).
    pub needs_setup: bool,
    pub detail: Option<String>,
}
