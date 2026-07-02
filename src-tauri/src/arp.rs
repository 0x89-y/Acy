
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ArpInfo {
    pub publisher: Option<String>,
    pub install_location: Option<String>,
}

pub fn norm(key: &str) -> String {
    key.trim().to_lowercase()
}

#[cfg(windows)]
pub fn scan() -> HashMap<String, ArpInfo> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_WOW: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    let hives = [
        (HKEY_LOCAL_MACHINE, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL_WOW),
        (HKEY_CURRENT_USER, UNINSTALL),
        (HKEY_CURRENT_USER, UNINSTALL_WOW),
    ];

    let mut map: HashMap<String, ArpInfo> = HashMap::new();

    let clean = |v: Result<String, _>| -> Option<String> {
        v.ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };

    for (root, path) in hives {
        let Ok(uninstall) = RegKey::predef(root).open_subkey(path) else {
            continue;
        };
        for sub_name in uninstall.enum_keys().flatten() {
            let Ok(sub) = uninstall.open_subkey(&sub_name) else {
                continue;
            };
            let info = ArpInfo {
                publisher: clean(sub.get_value("Publisher")),
                install_location: clean(sub.get_value("InstallLocation")),
            };
            if info.publisher.is_none() && info.install_location.is_none() {
                continue;
            }
            map.entry(norm(&sub_name)).or_insert(info);
        }
    }

    map
}

#[cfg(not(windows))]
pub fn scan() -> HashMap<String, ArpInfo> {
    HashMap::new()
}

#[cfg(windows)]
pub fn list_installed() -> Vec<crate::model::Package> {
    use crate::model::{Package, Source};
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_WOW: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    let hives = [
        (HKEY_LOCAL_MACHINE, UNINSTALL, "Machine", "X64"),
        (HKEY_LOCAL_MACHINE, UNINSTALL_WOW, "Machine", "X86"),
        (HKEY_CURRENT_USER, UNINSTALL, "User", "X64"),
        (HKEY_CURRENT_USER, UNINSTALL_WOW, "User", "X86"),
    ];

    let clean = |v: Result<String, _>| -> Option<String> {
        v.ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (root, path, scope, arch) in hives {
        let Ok(uninstall) = RegKey::predef(root).open_subkey(path) else {
            continue;
        };
        for sub_name in uninstall.enum_keys().flatten() {
            let Ok(sub) = uninstall.open_subkey(&sub_name) else {
                continue;
            };
            let Some(name) = clean(sub.get_value("DisplayName")) else {
                continue;
            };
            if sub.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1 {
                continue;
            }
            if sub.get_value::<String, _>("ParentKeyName").is_ok()
                || sub.get_value::<String, _>("ParentDisplayName").is_ok()
            {
                continue;
            }
            if matches!(
                clean(sub.get_value("ReleaseType")).as_deref(),
                Some("Security Update") | Some("Update") | Some("Hotfix")
            ) {
                continue;
            }
            let low = name.to_lowercase();
            if low.contains("(kb") || low.starts_with("security update") || low.starts_with("update for") {
                continue;
            }

            let id = format!("ARP\\{scope}\\{arch}\\{sub_name}");
            if !seen.insert(id.clone()) {
                continue;
            }
            let mut pkg = Package::new(id, name, Source::Winget);
            pkg.version = clean(sub.get_value("DisplayVersion"));
            pkg.publisher = clean(sub.get_value("Publisher"));
            pkg.install_location = clean(sub.get_value("InstallLocation"));
            pkg.installed = true;
            out.push(pkg);
        }
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

#[cfg(not(windows))]
pub fn list_installed() -> Vec<crate::model::Package> {
    Vec::new()
}
