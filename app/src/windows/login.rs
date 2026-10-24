//! V108-05 Twitch's login pages in the app's own windows ("Sign in with Twitch" in the app
//! window, "Connect Twitch pages", the code's activate page): only name + password + 2FA stay.
//! The Google / Apple / Amazon / Facebook buttons, the phone and "other device" options and
//! their dividers are hidden; "Sign up" stays. The window is sized to the login card.
//! V108-7U The same in 7TV's sign-in window. The activate page keeps its code field, Activate
//! and Log in; its banner, home link and footer go, and the window fits the form.
//! Styles and reads only; nothing here clicks.

use serde_json::Value;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewWindow};

/// Marker on `<html>` while a login form is on the page (set by `LOGIN_JS`); every rule below
/// is scoped to it, so other Twitch pages keep their own buttons.
pub const CSS: &str = concat!(
    "html[data-cyex-auth] :is(",
    "[data-a-target*=\"google\" i], [data-a-target*=\"apple\" i], [data-a-target*=\"amazon\" i], ",
    "[data-a-target*=\"facebook\" i], ",
    "[data-a-target*=\"other-device\" i], [data-a-target*=\"another-device\" i], ",
    "[data-test-selector*=\"google\" i], [data-test-selector*=\"apple\" i], ",
    "[data-test-selector*=\"amazon\" i], [data-test-selector*=\"facebook\" i], ",
    "[aria-label*=\"Google\"], [aria-label*=\"Apple\"], [aria-label*=\"Amazon\"], [aria-label*=\"Facebook\"], ",
    "[aria-label*=\"another device\" i], [aria-label*=\"QR\"], ",
    "a[href*=\"google.com\"], a[href*=\"apple.com\"], a[href*=\"amazon.com\"], a[href*=\"facebook.com\"], ",
    "a[href*=\"/oauth2/\"][href*=\"provider\"], ",
    // Cookie notices: hidden, never answered.
    "[data-a-target*=\"consent-banner\" i], [class*=\"consent-banner\" i], [class*=\"ConsentBanner\"], ",
    ".cyex-auth-hide) { display: none !important; } ",
    // Twitch's login buttons carry no id: the `screen` attribute plus the provider's logo image.
    "html[data-cyex-auth] :is(div, li):has(> :is(button, a)[screen] img:is([src*=\"google\"], [src*=\"apple\"], ",
    "[src*=\"amazon\"], [src*=\"facebook\"])), ",
    "html[data-cyex-auth] :is(button, a)[screen]:has(img:is([src*=\"google\"], [src*=\"apple\"], ",
    "[src*=\"amazon\"], [src*=\"facebook\"])) { display: none !important; } ",
    // The card sits a margin away from the window's edge (the page puts it flush to the top).
    "html[data-cyex-auth] .cyex-auth-card { margin-top: 24px !important; margin-bottom: 24px !important; }"
);

/// Marks the page, then hides by what the buttons say when the attributes above miss: a
/// third-party or other-device button inside the login card and the "or" line next to it.
/// Never touches the card's inputs, the submit button or the sign-up link.
pub const JS: &str = r#"
const authOnly = () => {
  const root = document.documentElement;
  if (!root || !document.body) return;
  const field = document.querySelector('input[type="password"], input[autocomplete="username"], input[name="username"], input[id^="login-username"], input[autocomplete="one-time-code"], input[name="code"], input[data-a-target*="verification"]');
  if (!field) {
    if (location.pathname.startsWith('/activate')) return activateOnly(root);
    root.removeAttribute('data-cyex-auth');
    delete root.dataset.cyexFit;
    return;
  }
  root.setAttribute('data-cyex-auth', '');
  delete root.dataset.cyexFit;
  // The card: the first box round the field with a background of its own.
  for (let c = field.parentElement, i = 0; c && c !== document.body && i < 20; c = c.parentElement, i++) {
    const bg = getComputedStyle(c).backgroundColor.match(/[\d.]+/g) || [];
    if (bg.length >= 3 && (bg.length < 4 || +bg[3] > 0)) {
      if (!c.classList.contains('cyex-auth-card')) { document.querySelectorAll('.cyex-auth-card').forEach((o) => o.classList.remove('cyex-auth-card')); c.classList.add('cyex-auth-card'); }
      break;
    }
  }
  let card = field;
  for (let i = 0; i < 14 && card.parentElement && card.parentElement !== document.body; i++) {
    card = card.parentElement;
    if (card.matches('[role="dialog"], [aria-modal="true"], main, form[class]')) break;
  }
  const other = /\b(google|apple|amazon|facebook|another device|other device|qr code|log ?in with phone|phone number login)\b/i;
  const keep = /sign ?up|create (an )?account|register|forgot|trouble|password|username|log ?in$|^log in$|submit|continue|verify/i;
  const list = [];
  for (const el of card.querySelectorAll('button, a, [role="button"], [role="tab"]')) {
    if (el.querySelector('input')) continue;
    const t = ((el.getAttribute('aria-label') || '') + ' ' + (el.textContent || '')).trim();
    if (!t || t.length > 80 || !other.test(t)) continue;
    if (/sign ?up|create (an )?account/i.test(t) && !/google|apple|amazon|facebook/i.test(t)) continue;
    if (keep.test(t.replace(other, '')) && !/google|apple|amazon|facebook/i.test(t) && el.type === 'submit') continue;
    list.push(el);
  }
  for (const el of list) {
    let box = el;
    while (box.parentElement && box.parentElement !== card && box.parentElement.querySelectorAll('button, a, input, [role="button"]').length === 1) box = box.parentElement;
    box.classList.add('cyex-auth-hide');
  }
  if (!list.length) return;
  // The "or" / "or log in with" line between the form and those buttons.
  for (const el of card.querySelectorAll('p, span, div, hr, h2, h3')) {
    if (el.children.length > 1) continue;
    const t = (el.textContent || '').trim();
    if (/^(or|oder|ou|o|или)$/i.test(t) || /^or\b.{0,24}(with|using)$/i.test(t) || /^(log ?in|sign ?in|continue) with$/i.test(t)) {
      el.classList.add('cyex-auth-hide');
      const p = el.parentElement;
      if (p && p !== card && p.children.length <= 3 && !p.querySelector('input, button, a')) p.classList.add('cyex-auth-hide');
    }
  }
};
// V108-7U The code's activate page: the code field, Activate and Log in stay; the banner with
// "Return To Twitch", the home link and the legal footer go. The window fits the form.
const activateOnly = (root) => {
  root.setAttribute('data-cyex-auth', '');
  const row = (el) => (el.closest('[class*="ScGrid"]') || el).classList.add('cyex-auth-hide');
  for (const a of document.querySelectorAll('main a[href="/"], main a[href="https://www.twitch.tv/"]')) if (/return to twitch/i.test(a.textContent || '')) row(a);
  for (const a of document.querySelectorAll('main a[href*="/legal/"]')) row(a);
  for (const a of document.querySelectorAll('a[aria-label="Twitch Home"]')) a.classList.add('cyex-auth-hide');
  const form = document.querySelector('main form');
  if (!form || !form.offsetWidth) { delete root.dataset.cyexFit; return; }
  root.dataset.cyexFit = JSON.stringify({ w: 440, h: Math.ceil(form.getBoundingClientRect().bottom + scrollY) });
};
let authTimer = 0;
new MutationObserver(() => { clearTimeout(authTimer); authTimer = setTimeout(authOnly, 40); })
  .observe(document, { childList: true, subtree: true });
"#;

/// The card only: "" while the page has no login card.
/// V108-7U Else the activate page's form (`data-cyex-fit`, from the top of the page).
const MEASURE: &str = r#"(() => { const c = document.querySelector('.cyex-auth-card'); return c && c.offsetWidth > 0 ? JSON.stringify({ w: c.offsetWidth, h: c.offsetHeight }) : (document.documentElement.dataset.cyexFit || ''); })()"#;

/// Room around the card, CSS px per side.
const MARGIN: f64 = 24.0;
const MIN: (f64, f64) = (360.0, 300.0);
/// Smaller changes than this don't move the window.
const STEP: f64 = 3.0;

/// The window size (logical px) for a login card `card`: card + margin, never below `MIN`, never
/// beyond the work area `work` (minus a little for the frame).
pub fn clamp(card: (f64, f64), margin: f64, work: (f64, f64)) -> (f64, f64) {
    let w = (card.0 + 2.0 * margin).max(MIN.0).min((work.0 - 16.0).max(MIN.0.min(work.0)));
    let h = (card.1 + 2.0 * margin).max(MIN.1).min((work.1 - 16.0).max(MIN.1.min(work.1)));
    (w.round(), h.round())
}

/// Whether the card changed enough since the size last set for the window to follow.
fn moved(old: Option<(f64, f64)>, new: (f64, f64)) -> bool {
    old.is_none_or(|o| (o.0 - new.0).abs() >= STEP || (o.1 - new.1).abs() >= STEP)
}

fn parse(answer: &str) -> Option<(f64, f64)> {
    let v: Value = serde_json::from_str(answer).ok()?;
    let inner = v.pointer("/result/value")?.as_str()?;
    if inner.is_empty() {
        return None;
    }
    let c: Value = serde_json::from_str(inner).ok()?;
    Some((c.get("w")?.as_f64()?, c.get("h")?.as_f64()?))
}

/// Keeps the window `label` fitted to its login card until it closes (or `until`): measures
/// the page twice a second and resizes (centred on the app window) when the card changes, e.g.
/// on the 2FA step. Pages without a card (consent, code, other Twitch pages) get `rest`, the
/// window's usual size, back. Only for windows the app opened; the page itself has no IPC.
pub fn follow(app: &AppHandle, label: String, rest: (f64, f64), until: std::time::Duration) {
    let app = app.clone();
    let end = std::time::Instant::now() + until;
    tauri::async_runtime::spawn(async move {
        let mut last: Option<(f64, f64)> = None;
        let mut applied = 0;
        let mut gone = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(450)).await;
            let Some(w) = app.get_webview_window(&label) else { return };
            if std::time::Instant::now() > end {
                return;
            }
            let params = format!(r#"{{"expression":{},"returnByValue":true}}"#, Value::String(MEASURE.into()));
            let Some(answer) = crate::sys::devtools::call(&w, "Runtime.evaluate", params).await else {
                continue;
            };
            let Some(card) = parse(&answer) else {
                // No card for two polls in a row (a page change, not a re-render): usual size.
                gone += 1;
                if gone == 2 && last.take().is_some() {
                    place(&app, &w, rest, 0.0);
                }
                continue;
            };
            gone = 0;
            if !moved(last, card) || applied > 40 {
                continue;
            }
            last = Some(card);
            applied += 1;
            place(&app, &w, card, MARGIN);
        }
    });
}

fn place(app: &AppHandle, w: &WebviewWindow, card: (f64, f64), margin: f64) {
    let Ok(scale) = w.scale_factor() else { return };
    let main = app.get_webview_window("main");
    let monitor = main.as_ref().and_then(|m| m.current_monitor().ok().flatten()).or_else(|| w.current_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let area = *monitor.work_area();
    let (aw, ah) = (f64::from(area.size.width) / scale, f64::from(area.size.height) / scale);
    let (iw, ih) = clamp(card, margin, (aw, ah));
    // The frame (title bar, borders) around the page, read before the resize takes effect.
    let frame = match (w.outer_size(), w.inner_size()) {
        (Ok(o), Ok(i)) => (f64::from(o.width.saturating_sub(i.width)) / scale, f64::from(o.height.saturating_sub(i.height)) / scale),
        _ => (0.0, 0.0),
    };
    let _ = w.set_size(LogicalSize::new(iw, ih));
    let outer = (iw + frame.0, ih + frame.1);
    let (ox, oy, ow, oh) = match main.as_ref().and_then(|m| Some((m.outer_position().ok()?, m.outer_size().ok()?))) {
        Some((p, s)) => (f64::from(p.x) / scale, f64::from(p.y) / scale, f64::from(s.width) / scale, f64::from(s.height) / scale),
        None => (f64::from(area.position.x) / scale, f64::from(area.position.y) / scale, aw, ah),
    };
    let (ax, ay) = (f64::from(area.position.x) / scale, f64::from(area.position.y) / scale);
    let x = (ox + (ow - outer.0) / 2.0).clamp(ax, (ax + aw - outer.0).max(ax));
    let y = (oy + (oh - outer.1) / 2.0).clamp(ay, (ay + ah - outer.1).max(ay));
    let _ = w.set_position(LogicalPosition::new(x.round(), y.round()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_hugs_the_card_within_bounds() {
        assert_eq!(clamp((400.0, 520.0), MARGIN, (1920.0, 1040.0)), (448.0, 568.0));
        // Tiny cards keep a sane minimum.
        assert_eq!(clamp((100.0, 80.0), MARGIN, (1920.0, 1040.0)), (360.0, 300.0));
        // Never beyond the work area.
        assert_eq!(clamp((3000.0, 3000.0), MARGIN, (1280.0, 720.0)), (1264.0, 704.0));
        // A work area below the minimum doesn't ask for more than it has.
        assert_eq!(clamp((400.0, 400.0), MARGIN, (320.0, 280.0)), (320.0, 280.0));
        // The usual size back (no margin) keeps its size when it fits.
        assert_eq!(clamp((520.0, 760.0), 0.0, (1920.0, 1040.0)), (520.0, 760.0));
    }

    #[test]
    fn small_card_changes_are_ignored() {
        assert!(moved(None, (400.0, 500.0)));
        assert!(!moved(Some((400.0, 500.0)), (402.0, 501.0)));
        assert!(moved(Some((400.0, 500.0)), (400.0, 560.0)));
    }

    #[test]
    fn the_answer_is_read() {
        let a = r#"{"result":{"type":"string","value":"{\"w\":401,\"h\":523}"}}"#;
        assert_eq!(parse(a), Some((401.0, 523.0)));
        assert_eq!(parse(r#"{"result":{"type":"string","value":""}}"#), None);
        assert_eq!(parse("nope"), None);
    }

    #[test]
    fn the_rules_only_hide_and_never_click() {
        assert!(CSS.starts_with("html[data-cyex-auth]") && CSS.contains("google") && CSS.contains("another device"));
        assert!(CSS.contains("[screen]:has(img:is([src*=\"google\"]") && CSS.contains(".cyex-auth-card"));
        assert!(CSS.matches('{').count() == CSS.matches('}').count());
        assert!(!CSS.to_lowercase().contains("sign up") && !CSS.to_lowercase().contains("create"));
        assert!(JS.contains("sign ?up|create") && !JS.contains(".click(") && !MEASURE.contains("click"));
        // V108-7U Cookie notices are hidden, not answered; the activate page keeps its form.
        assert!(CSS.contains("consent-banner") && JS.contains("activateOnly") && MEASURE.contains("cyexFit"));
        assert!(!JS.contains("form.classList") && JS.contains("return to twitch"));
    }
}
