//! 3.8, 1.27, 1.28 Twitch's own pages (payment methods, checkout, subscriptions) in an app
//! window with the cyex skin, for what Twitch's API doesn't do. Each account gets its own
//! WebView2 profile folder, so its Twitch sign-in stays separate and is remembered. The
//! page gets no app IPC (remote origin, no capability lists it). Links that leave Twitch
//! open in the browser; payment pop-ups (PayPal, card checks) open as small windows.
//! V103-01 Sign-in codes are approved here too, never in the browser.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use cyex_core::color::Theme;
use cyex_core::palette;
use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::theme::{self, ThemeLook};

const PREFIX: &str = "twitch-";
/// V103-01 The profile a sign-in for a new account is approved in. The account takes it
/// over once it's added, so its Twitch login is kept for payment methods and the like.
const NEW: &str = "new";

/// Opens `url` (a twitch.tv page) in the account's Twitch window, reusing it when open.
pub async fn open(app: &AppHandle, account: &str, url: &str) -> Result<(), String> {
    let url = twitch_url(url)?;
    let accounts = crate::auth::accounts(app);
    let acc = accounts
        .get(account)
        .ok_or("That account isn’t signed in here.")?;
    let name = if acc.display_name.is_empty() {
        acc.login.clone()
    } else {
        acc.display_name.clone()
    };
    show(app, account, &format!("Twitch · {name}"), url)
}

/// V103-01 Twitch's page for approving a sign-in code: the account's Twitch window when
/// the sign-in is added to an account that's here, else the new-sign-in window.
pub async fn open_activation(app: &AppHandle, account: Option<&str>, url: &str) -> Result<(), String> {
    let url = twitch_url(url)?;
    if !is_activation(&url) {
        return Err("That isn’t Twitch’s sign-in page.".into());
    }
    match account.filter(|a| crate::auth::accounts(app).get(a).is_some()) {
        Some(account) => open(app, account, url.as_str()).await,
        None => show(app, NEW, "Twitch", url),
    }
}

/// V103-01 A sign-in ended. Its window closes if it still shows the approval (Twitch moves
/// it to its connections page once approved). A new account (`signed_in`) takes over the
/// new-sign-in window's profile and its Twitch login, unless it has a profile already.
/// Without one, the new-sign-in profile is cleared so no login is left behind.
pub fn sign_in_done(app: &AppHandle, account: Option<&str>, signed_in: Option<&str>) {
    let existing = account.filter(|a| crate::auth::accounts(app).get(a).is_some());
    if let Some(account) = existing {
        if let Some(w) = app.get_webview_window(&format!("{PREFIX}{account}"))
            && w.url().is_ok_and(|u| closes_after_sign_in(&u))
        {
            let _ = w.close();
        }
        return;
    }
    if let Some(w) = app.get_webview_window(&format!("{PREFIX}{NEW}")) {
        let _ = w.close();
    }
    let Some(from) = profile_dir(app, NEW) else {
        return;
    };
    match signed_in.and_then(|id| profile_dir(app, id)) {
        // WebView2 lets go of the folder a moment after its window closed.
        Some(to) => std::thread::spawn(move || retry(|| adopt_dir(&from, &to))),
        None => std::thread::spawn(move || retry(|| remove_dir(&from))),
    };
}

/// Tries `f` for up to 10 s while the folder is still in use.
fn retry(f: impl Fn() -> std::io::Result<()>) {
    for _ in 0..40 {
        if f().is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

fn remove_dir(dir: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Moves the new-sign-in profile to the account's, or drops it when the account has one.
fn adopt_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    if !from.exists() {
        return Ok(());
    }
    if to.exists() {
        return remove_dir(from);
    }
    std::fs::rename(from, to)
}

fn twitch_url(url: &str) -> Result<Url, String> {
    let url: Url = url.parse().map_err(|_| "That isn’t a Twitch page.")?;
    if url.scheme() != "https" || !is_twitch(&url) {
        return Err("That isn’t a Twitch page.".into());
    }
    Ok(url)
}

/// `twitch.tv/activate`, where sign-in codes are approved.
fn is_activation(url: &Url) -> bool {
    is_twitch(url) && url.path().trim_end_matches('/') == "/activate"
}

/// Still on the approval, or on the page Twitch shows after it.
fn closes_after_sign_in(url: &Url) -> bool {
    is_activation(url) || (is_twitch(url) && url.path().starts_with("/settings/connections"))
}

/// Shows `url` in the Twitch window for `profile`, reusing it when open.
fn show(app: &AppHandle, profile: &str, title: &str, url: Url) -> Result<(), String> {
    let label = format!("{PREFIX}{profile}");
    if let Some(w) = app.get_webview_window(&label) {
        w.navigate(url).map_err(|e| e.to_string())?;
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    let look = theme::look(app);
    let dir = profile_dir(app, profile).ok_or("Couldn’t find the app data folder.")?;
    let nav_app = app.clone();
    let pop_app = app.clone();
    WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title(title)
        .inner_size(1100.0, 820.0)
        .min_inner_size(480.0, 420.0)
        .theme(Some(match look.theme {
            Theme::Dark => tauri::Theme::Dark,
            Theme::Light => tauri::Theme::Light,
        }))
        .background_color(theme::background(app))
        .data_directory(dir)
        .initialization_script(skin(&look))
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

/// The account's Twitch web profile folder; 7TV's window borrows it for "both" (V1051-02).
pub(crate) fn profile_dir(app: &AppHandle, account: &str) -> Option<PathBuf> {
    let safe: String = account.chars().filter(char::is_ascii_alphanumeric).collect();
    (!safe.is_empty()).then(|| {
        crate::paths::local(app).ok().map(|d| d.join("twitch").join(safe))
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

/// V107-Mz: the Bits page (`/bits`) shows only the packages: the info text, FAQ, banner and
/// promo callouts go. Twitch's own stable `bits-landing-page*` class names, no hashed ones;
/// they only exist on that page, so no path check is needed.
const BITS_ONLY: &str = concat!(
    // Promo column (power-ups image, text, list, links), banner, FAQ, callouts: gone, whatever
    // the wrapper is called, by class, by the `how-to-cheer` anchor and by the banner image.
    ".bits-landing-page__content--left-pane, .bits-landing-page__gems-banner, .bits-landing-page-faq, ",
    ".bits-landing-page-cheermote-description, .bits-landing-page-text-section, ",
    "[class*=\"subtember-callout\"], #how-to-cheer, div:has(> #how-to-cheer), ",
    "img[src*=\"power-ups-animated-banner\"], .bits-landing-page :is(figure, picture):has(img[src*=\"power-ups\"]) ",
    "{ display: none !important; } ",
    // What stays: one centred column, normal flow (no inline-block or float next to the promo).
    ".bits-landing-page__content, .bits-landing-page__content > div, .bits-landing-page__content--right-pane, ",
    ".bits-landing-page-right-pane { display: block !important; float: none !important; position: static !important; ",
    "width: 100% !important; max-width: 720px !important; margin-left: auto !important; margin-right: auto !important; ",
    "padding-left: 0 !important; padding-right: 0 !important; box-sizing: border-box !important; } ",
    ".bits-landing-page-right-pane__buy-bits-title-container { display: flex !important; align-items: center !important; ",
    "justify-content: space-between !important; text-align: left !important; }"
);

/// V107-H12: fallback when Twitch's class names differ (hashed): on `/bits` only, finds the
/// package price buttons, the "Buy Bits" heading and the balance by their text, hides every
/// sibling subtree (up to 6 levels above their common container) that holds none of them,
/// and lays the kept column out at 720 px centred. Reads and styles only; never clicks.
const BITS_FALLBACK_JS: &str = r#"
const bitsOnly = () => {
  if (!location.pathname.startsWith('/bits') || !document.body) return;
  const price = /(^|\s)(US\$|\$|€|£)\s?\d|\d\s?(€|£|USD|EUR)(\s|$)/;
  const keep = [];
  for (const b of document.body.querySelectorAll('button, a')) {
    const t = (b.textContent || '').trim();
    if (t.length < 24 && price.test(t)) keep.push(b);
  }
  if (!keep.length) return;
  for (const h of document.body.querySelectorAll('h1, h2, h3, p, span, div')) {
    if (h.children.length) continue;
    const t = (h.textContent || '').trim();
    if (/^Buy Bits$/i.test(t) || /^You have\b.*\bBits?\b/i.test(t)) keep.push(h);
  }
  let box = keep[0];
  while (box && !keep.every((k) => box.contains(k))) box = box.parentElement;
  if (!box) return;
  const chain = [];
  let up = box;
  for (let i = 0; i < 6 && up && up !== document.body; i++) { chain.push(up); up = up.parentElement; }
  const inChain = new Set(chain);
  for (const el of chain) {
    el.style.setProperty('width', '100%', 'important');
    el.style.setProperty('float', 'none', 'important');
    el.style.setProperty('position', 'static', 'important');
    el.style.setProperty('box-sizing', 'border-box', 'important');
    const par = el.parentElement;
    if (!par || par === document.body) continue;
    if (el !== box) { el.style.setProperty('max-width', '100%', 'important'); }
    for (const sib of par.children) {
      if (inChain.has(sib) || /^(SCRIPT|STYLE|LINK|META)$/.test(sib.tagName)) continue;
      if (keep.some((k) => sib.contains(k))) continue;
      sib.style.setProperty('display', 'none', 'important');
    }
  }
  box.style.setProperty('max-width', '720px', 'important');
  box.style.setProperty('margin', '0 auto', 'important');
  box.style.setProperty('display', 'block', 'important');
};
let bitsTimer = 0;
new MutationObserver(() => { clearTimeout(bitsTimer); bitsTimer = setTimeout(bitsOnly, 200); })
  .observe(document, { childList: true, subtree: true });
"#;

/// The skin: Twitch's theme variables set to the app's tokens (`ui/src/design/tokens.css`),
/// the accent in place of Twitch purple, small corners, the app font, no side or top nav.
/// Runs in every frame, so it checks it's Twitch's top page before touching anything.
pub fn skin(look: &ThemeLook) -> String {
    let theme = look.theme;
    let a = &look.accent;
    // A custom theme's colors (U8), else the built-in ones.
    let p = look
        .palette
        .clone()
        .unwrap_or_else(|| palette::palette(theme, &Default::default()));
    let token = |k: &str| p.tokens.get(k).map_or("", String::as_str);
    let (bg, surface, elevated, elevated2, border, text, muted) = (
        token("bg"),
        token("surface"),
        token("elevated"),
        token("elevated-2"),
        token("border"),
        token("text"),
        token("muted"),
    );
    let (mix_to, twilight) = match theme {
        Theme::Dark => ("black", 1),
        Theme::Light => ("white", 0),
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
    let css = format!("{css} {BITS_ONLY}");
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
         add(); {bits} }})();",
        bits = BITS_FALLBACK_JS
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
    fn sign_in_pages() {
        let u = |s: &str| s.parse::<Url>().unwrap();
        assert!(is_activation(&u(
            "https://www.twitch.tv/activate?device-code=ABCDEFGH"
        )));
        assert!(!is_activation(&u("https://www.twitch.tv/activated")));
        assert!(!is_activation(&u("https://evil.example/activate")));
        assert!(closes_after_sign_in(&u(
            "https://www.twitch.tv/settings/connections"
        )));
        assert!(!closes_after_sign_in(&u("https://www.twitch.tv/wallet")));
    }

    #[test]
    fn a_new_account_takes_over_the_sign_in_profile() {
        let base = std::env::temp_dir().join(format!("cyex-adopt-{}", std::process::id()));
        let (new, mine, theirs) = (base.join("new"), base.join("1"), base.join("2"));
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("login"), "a").unwrap();
        adopt_dir(&new, &mine).unwrap();
        assert!(!new.exists() && mine.join("login").exists());
        // An account with a profile of its own keeps it; the new one goes.
        std::fs::create_dir_all(&new).unwrap();
        std::fs::create_dir_all(&theirs).unwrap();
        adopt_dir(&new, &theirs).unwrap();
        assert!(!new.exists() && theirs.exists());
        // Nothing to move: fine.
        adopt_dir(&new, &mine).unwrap();
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn skin_is_one_guarded_script() {
        let look = ThemeLook {
            theme: Theme::Dark,
            palette: None,
            accent: cyex_core::color::accent_tokens("#145aff", Theme::Dark),
            wallpaper: None,
        };
        let js = skin(&look);
        assert!(js.contains("--color-background-body: #1e1f23"));
        assert!(js.starts_with("(() => {") && js.contains("window.top !== window"));
        assert!(js.contains("--color-twitch-purple-9: #") && js.contains("twilight.theme', '1'"));
        // V107-Mz: the Bits page keeps the packages only; the script stays valid JSON-in-JS.
        assert!(js.contains("bits-landing-page__content--left-pane") && js.contains("bits-landing-page-faq"));
        assert!(js.contains("#how-to-cheer") && js.contains("power-ups-animated-banner"));
        assert!(js.contains("startsWith('/bits')") && js.contains("Buy Bits") && !js.contains("click("));
        assert!(BITS_ONLY.matches('{').count() == BITS_ONLY.matches('}').count());
    }
}
