//! Native title bar styling. On Windows 11 the window can get a system backdrop
//! (Mica, Mica Alt or Acrylic), which the title bar and the (transparent) sidebar
//! share. Without one, the title bar is painted the app's shell colour so it
//! still blends in.

use tauri::window::{Effect, EffectsBuilder};
use tauri::{Theme, WebviewWindow};

/// Mica and custom caption colours need Windows 11 (build 22000+).
#[cfg(windows)]
fn is_windows_11() -> bool {
    use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .is_some_and(|build| build >= 22000)
}

/// "Show accent color on title bars and window borders" (Settings → Personalization
/// → Colors). With it on, Windows paints native title bars in the accent colour and
/// Mica can't show through them, so Acy paints its own colour instead.
#[cfg(windows)]
fn accent_title_bars() -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\DWM")
        .and_then(|k| k.get_value::<u32, _>("ColorPrevalence"))
        .is_ok_and(|v| v == 1)
}

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim().strip_prefix('#')?;
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// Sets the title bar colour, or restores the system's with `None`.
#[cfg(windows)]
fn set_caption_color(window: &WebviewWindow, rgb: Option<(u8, u8, u8)>) {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR};
    let Ok(hwnd) = window.hwnd() else { return };
    // COLORREF is 0x00BBGGRR; DWMWA_COLOR_DEFAULT (0xFFFFFFFF) means "system colour".
    let color: u32 = match rgb {
        Some((r, g, b)) => (b as u32) << 16 | (g as u32) << 8 | r as u32,
        None => 0xFFFF_FFFF,
    };
    // SAFETY: hwnd is this window's live handle; the attribute is a 4-byte COLORREF.
    unsafe {
        DwmSetWindowAttribute(
            hwnd.0 as _,
            DWMWA_CAPTION_COLOR as u32,
            &color as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

/// Applies the app theme to the native title bar. `theme` is "dark", "light",
/// or None to follow Windows (the setting is "System"). `backdrop` is "off",
/// "mica", "mica-alt" or "acrylic". Returns whether a backdrop is active, so the
/// frontend knows to make its shell transparent.
#[tauri::command]
pub fn set_window_style(
    window: WebviewWindow,
    theme: Option<String>,
    dark: bool,
    backdrop: String,
    caption: String,
) -> bool {
    // Pinning the theme also changes what the page sees as prefers-color-scheme,
    // so "System" must stay None or switching back to it would stick.
    let pinned = match theme.as_deref() {
        Some("dark") => Some(Theme::Dark),
        Some("light") => Some(Theme::Light),
        _ => None,
    };
    let _ = window.set_theme(pinned);

    #[cfg(windows)]
    {
        let win11 = is_windows_11();
        // On Windows 11 22H2+ all three are DWM system backdrops covering the
        // title bar too (Acrylic = DWMSBT_TRANSIENTWINDOW, so no drag lag).
        let effect = match (backdrop.as_str(), dark) {
            ("mica", true) => Some(Effect::MicaDark),
            ("mica", false) => Some(Effect::MicaLight),
            ("mica-alt", true) => Some(Effect::TabbedDark),
            ("mica-alt", false) => Some(Effect::TabbedLight),
            ("acrylic", _) => Some(Effect::Acrylic),
            _ => None,
        };
        // Only when the backdrop can reach the title bar too; otherwise the title
        // bar (accent-coloured) and the translucent sidebar wouldn't match.
        if let Some(effect) = effect.filter(|_| win11 && !accent_title_bars()) {
            let _ = window.set_effects(EffectsBuilder::new().effect(effect).build());
            set_caption_color(&window, None);
            return true;
        }
        let _ = window.set_effects(None::<tauri::utils::config::WindowEffectsConfig>);
        if win11 {
            set_caption_color(&window, parse_hex(&caption));
        }
        false
    }
    #[cfg(not(windows))]
    {
        let _ = (dark, backdrop, caption, parse_hex, Effect::Mica, EffectsBuilder::new());
        false
    }
}

#[cfg(test)]
mod tests {
    use super::parse_hex;

    #[test]
    fn parses_caption_colours() {
        assert_eq!(parse_hex("#161619"), Some((0x16, 0x16, 0x19)));
        assert_eq!(parse_hex(" #F4F4F5 "), Some((0xf4, 0xf4, 0xf5)));
        assert_eq!(parse_hex("161619"), None);
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(parse_hex("#zzzzzz"), None);
    }
}
