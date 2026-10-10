//! 3.8, 1.27, 1.28 Twitch's own pages (payment methods, checkout, subscriptions) in an app
//! window with the cyex skin, for what Twitch's API doesn't do. Each account gets its own
//! WebView2 profile folder, so its Twitch sign-in stays separate and is remembered. The
//! page gets no app IPC (remote origin, no capability lists it). Links that leave Twitch
//! open in the browser; payment pop-ups (PayPal, card checks) open as small windows.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use cyex_core::color::{Theme, accent_tokens};
use cyex_settings::Partition;
use cyex_settings::schema::Appearance;
use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::window::Color;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::settings::Settings;

const PREFIX: &str = "twitch-";

/// Opens `url` (a twitch.tv page) in the account's Twitch window, reusing it when open.
pub async fn open(app: &AppHandle, account: &str, url: &str) -> Result<(), String> {
    let url: Url = url.parse().map_err(|_| "That isn't a Twitch page.")?;
    if url.scheme() != "https" || !is_twitch(&url) {
        return Err("That isn't a Twitch page.".into());
    }
    let accounts = crate::auth::accounts(app);
    let acc = accounts
        .get(account)
        .ok_or("That account isn't signed in here.")?;
    let name = if acc.display_name.is_empty() {
        acc.login.clone()
    } else {
        acc.display_name.clone()
    };
    let label = format!("{PREFIX}{account}");
    if let Some(w) = app.get_webview_window(&label) {
        w.navigate(url).map_err(|e| e.to_string())?;
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    let (theme, accent) = look(app);
    let dir = profile_dir(app, account).ok_or("Couldn't find the app data folder.")?;
    let nav_app = app.clone();
    let pop_app = app.clone();
    WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title(format!("Twitch · {name}"))
        .inner_size(1100.0, 820.0)
        .min_inner_size(480.0, 420.0)
        .theme(Some(match theme {
            Theme::Dark => tauri::Theme::Dark,
            Theme::Light => tauri::Theme::Light,
        }))
        .background_color(background(theme))
        .data_directory(dir)
        .initialization_script(skin(theme, &accent))
        .on_navigation(move |u| {
            let ok = is_twitch(u) || is_payment(u) || u.scheme() == "about";
            if !ok && u.scheme() == "https" {
                let _ = nav_app.opener().open_url(u.as_str(), None::<&str>);
            }
            ok
        })
        .on_new_window(move |u, features| popup(&pop_app, u, features))
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Signs the account out of Twitch's pages: clears its profile.
pub fn sign_out(app: &AppHandle, account: &str) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(&format!("{PREFIX}{account}")) {
        return w.clear_all_browsing_data().map_err(|e| e.to_string());
    }
    forget(app, account);
    Ok(())
}

/// An account was removed: its profile folder goes too (once its window is closed).
pub fn forget(app: &AppHandle, account: &str) {
    if let Some(w) = app.get_webview_window(&format!("{PREFIX}{account}")) {
        let _ = w.close();
    }
    if let Some(dir) = profile_dir(app, account) {
        let _ = std::fs::remove_dir_all(dir);
    }
}

fn profile_dir(app: &AppHandle, account: &str) -> Option<PathBuf> {
    let safe: String = account.chars().filter(char::is_ascii_alphanumeric).collect();
    (!safe.is_empty()).then(|| {
        app.path()
            .local_data_dir()
            .ok()
            .map(|d| d.join("cyex-viewer").join("twitch").join(safe))
    })?
}

/// Payment and sign-in pop-ups: a small window sharing the page's profile, so the
/// opener keeps talking to it. Anything else opens in the browser.
fn popup(app: &AppHandle, url: Url, features: NewWindowFeatures) -> NewWindowResponse<tauri::Wry> {
    if !(is_twitch(&url) || is_payment(&url) || url.scheme() == "about") {
        if url.scheme() == "https" {
            let _ = app.opener().open_url(url.as_str(), None::<&str>);
        }
        return NewWindowResponse::Deny;
    }
    static NEXT: AtomicU32 = AtomicU32::new(1);
    let label = format!("{PREFIX}pop-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let built = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External("about:blank".parse().expect("valid")),
    )
    .window_features(features)
    .title(url.host_str().unwrap_or("Twitch"))
    .on_document_title_changed(|w, title| {
        let _ = w.set_title(&title);
    })
    // Bank checks (3-D Secure) can go anywhere on https.
    .on_navigation(|u| u.scheme() == "https" || u.scheme() == "about")
    .build();
    match built {
        Ok(window) => NewWindowResponse::Create { window },
        Err(e) => {
            eprintln!("twitch window: pop-up failed: {e}");
            NewWindowResponse::Deny
        }
    }
}

fn host_is(url: &Url, domain: &str) -> bool {
    url.host_str()
        .is_some_and(|h| h == domain || h.ends_with(&format!(".{domain}")))
}

fn is_twitch(url: &Url) -> bool {
    host_is(url, "twitch.tv")
}

/// Hosts Twitch's checkout and sign-in send the page to: PayPal, Braintree, Xsolla,
/// Recurly, Amazon (paying and signing in), Google and Apple sign-in.
fn is_payment(url: &Url) -> bool {
    [
        "paypal.com",
        "braintreegateway.com",
        "xsolla.com",
        "recurly.com",
        "amazon.com",
        "amazon.de",
        "amazon.co.uk",
        "accounts.google.com",
        "appleid.apple.com",
    ]
    .iter()
    .any(|d| host_is(url, d))
}

fn look(app: &AppHandle) -> (Theme, String) {
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    (a.theme, a.accent)
}

fn background(theme: Theme) -> Color {
    match theme {
        Theme::Light => Color(0xf3, 0xf3, 0xf4, 0xff),
        Theme::Dark => Color(0x1e, 0x1f, 0x23, 0xff),
    }
}

/// The skin: Twitch's theme variables set to the app's tokens (`ui/src/design/tokens.css`),
/// the accent in place of Twitch purple, small corners, the app font, no side or top nav.
/// Runs in every frame, so it checks it's Twitch's top page before touching anything.
pub fn skin(theme: Theme, accent: &str) -> String {
    let a = accent_tokens(accent, theme);
    let (bg, surface, elevated, elevated2, border, text, muted, mix_to, twilight) = match theme {
        Theme::Dark => (
            "#1e1f23", "#26272c", "#2f3037", "#353640", "#34353c", "#e8e8ea", "#b4b5bb", "black", 1,
        ),
        Theme::Light => (
            "#f3f3f4", "#ffffff", "#ffffff", "#f4f4f6", "#dcdde1", "#1b1c20", "#4f515a", "white", 0,
        ),
    };
    let (fill, hover, link, ink) = (&a.accent, &a.accent_hover, &a.accent_text, &a.accent_ink);
    // Twitch's accent scale (`--color-accent-primary-1..5`): dark pages use 4 for fills and 5
    // for text, light ones 2 for both.
    let primary = match theme {
        Theme::Dark => [
            format!("color-mix(in srgb, {link} 70%, white)"),
            link.clone(),
            hover.clone(),
            fill.clone(),
            link.clone(),
        ],
        Theme::Light => [
            format!("color-mix(in srgb, {link} 80%, black)"),
            link.clone(),
            hover.clone(),
            fill.clone(),
            fill.clone(),
        ],
    };
    let font = "'IBM Plex Sans', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif";
    let mut vars: Vec<(String, String)> = [
        ("background-body", bg),
        ("background-base", surface),
        ("background-alt", elevated),
        ("background-alt-2", elevated2),
        ("background-float", elevated),
        ("border-base", border),
        ("text-base", text),
        ("text-alt", text),
        ("text-alt-2", muted),
    ]
    .iter()
    .map(|(k, v)| (format!("color-{k}"), (*v).to_owned()))
    .collect();
    for k in [
        "text-link",
        "text-link-hover",
        "text-link-active",
        "text-link-focus",
        "text-link-visited",
        "text-button-text",
    ] {
        vars.push((format!("color-{k}"), link.clone()));
    }
    for k in [
        "twitch-purple",
        "accent",
        "fill-brand",
        "border-brand",
        "background-accent",
        "background-button-primary-default",
        "background-button-primary-active",
        "background-button-brand",
        "border-input-focus",
        "border-input-checkbox-focus",
        "border-input-checkbox-checked",
        "background-input-checked",
        "background-input-checkbox-checked",
        "background-progress-status",
        "background-range-fill",
        "border-range-handle",
        "background-interactable-selected",
    ] {
        vars.push((format!("color-{k}"), fill.clone()));
    }
    for k in [
        "background-button-primary-hover",
        "background-button-brand-hover",
        "background-input-checkbox-checked-hover",
    ] {
        vars.push((format!("color-{k}"), hover.clone()));
    }
    vars.push(("color-text-button-primary".into(), ink.clone()));
    for (i, v) in primary.iter().enumerate() {
        vars.push((format!("color-accent-primary-{}", i + 1), v.clone()));
    }
    // 1-8 deep, 9 the fill, 10 hover, 11 text, 12-15 light.
    for (n, pct) in [
        (1, 15),
        (2, 22),
        (3, 30),
        (4, 40),
        (5, 52),
        (6, 64),
        (7, 76),
        (8, 88),
    ] {
        vars.push((
            format!("color-twitch-purple-{n}"),
            format!("color-mix(in srgb, {fill} {pct}%, {mix_to})"),
        ));
    }
    vars.push(("color-twitch-purple-9".into(), fill.clone()));
    vars.push(("color-twitch-purple-10".into(), hover.clone()));
    vars.push(("color-twitch-purple-11".into(), link.clone()));
    for (n, pct) in [(12, 75), (13, 55), (14, 35), (15, 18)] {
        vars.push((
            format!("color-twitch-purple-{n}"),
            format!("color-mix(in srgb, {link} {pct}%, white)"),
        ));
    }
    for k in ["border-radius", "button-border-radius", "input-border-radius"] {
        for size in ["small", "default", "medium", "large"] {
            vars.push((format!("{k}-{size}"), "3px".into()));
        }
    }
    vars.push(("font-base".into(), font.into()));
    vars.push(("font-display".into(), font.into()));
    let vars: String = vars
        .iter()
        .map(|(k, v)| format!("--{k}: {v} !important;"))
        .collect::<Vec<_>>()
        .join(" ");
    // Twitch sets its tokens on `:root` and again on regions and buttons that override
    // them; ours win on all of those.
    let css = format!(
        "html:root, html body, html body .tw-root--theme-dark, html body .tw-root--theme-light,          [class*=\"ScTokenOverrideCSSVars\"], [class*=\"ScAccentRegionCssVars\"],          [class*=\"ScCoreButton\"] {{ {vars} }}          html body {{ background: {bg} !important; }}          .top-nav, [data-a-target=\"top-nav-container\"], #side-nav, [data-test-selector=\"side-nav\"],          .side-nav, #twilight-sticky-footer-root, .persistent-player {{ display: none !important; }}"
    );
    let css = serde_json::to_string(&css).unwrap_or_default();
    // Runs before the page has a root element: waits for one, and puts the style back if
    // the page drops it.
    format!(
        "(() => {{ if (window.top !== window || !/(^|\\.)twitch\\.tv$/.test(location.hostname)) return; \
         try {{ localStorage.setItem('twilight.theme', '{twilight}'); }} catch {{}} \
         const add = () => {{ if (document.getElementById('cyex-skin')) return; \
         const root = document.head || document.documentElement; if (!root) return; \
         const s = document.createElement('style'); s.id = 'cyex-skin'; s.textContent = {css}; \
         root.appendChild(s); }}; \
         new MutationObserver(add).observe(document, {{ childList: true, subtree: true }}); \
         add(); }})();"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        let u = |s: &str| s.parse::<Url>().unwrap();
        assert!(is_twitch(&u("https://www.twitch.tv/wallet")) && is_twitch(&u("https://twitch.tv/")));
        assert!(!is_twitch(&u("https://twitch.tv.evil.com/")) && !is_twitch(&u("https://nottwitch.tv/")));
        assert!(is_payment(&u("https://www.paypal.com/checkoutnow")));
    }

    #[test]
    fn skin_is_one_guarded_script() {
        let js = skin(Theme::Dark, "#145aff");
        assert!(js.starts_with("(() => {") && js.contains("window.top !== window"));
        assert!(js.contains("--color-twitch-purple-9: #") && js.contains("twilight.theme', '1'"));
    }
}
