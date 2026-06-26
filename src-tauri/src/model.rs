use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Winget,
    Scoop,
    Choco,
    Msstore,
    Local,
}

impl Source {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub id: String,
    pub name: String,
    pub source: Source,
    pub version: Option<String>,
    pub available_version: Option<String>,
    pub publisher: Option<String>,
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
            homepage: None,
            description: None,
            installed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub name: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub installed: bool,
    pub variants: Vec<Package>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerStatus {
    pub source: Source,
    pub available: bool,
    pub needs_setup: bool,
    pub detail: Option<String>,
}
