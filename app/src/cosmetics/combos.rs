//! V1051-04 The saved combos, from settings.

use cyex_settings::Partition;
use cyex_settings::schema::{Combo, ComboDone, ComboError, ComboOp, Combos};
use tauri::{AppHandle, Manager};

use crate::settings::Settings;

fn all(app: &AppHandle) -> Combos {
    app.state::<Settings>().read::<Combos>(Partition::Combos)
}

pub fn list(app: &AppHandle, account: &str) -> Vec<Combo> {
    all(app).list(account).to_vec()
}

pub fn find(app: &AppHandle, account: &str, name: &str) -> Option<Combo> {
    all(app).find(account, name).cloned()
}

/// Runs one op through `Settings::update` (saved and sent to every window).
pub fn change(app: &AppHandle, account: &str, op: ComboOp) -> Result<ComboDone, ComboError> {
    let mut result = None;
    let saved = app
        .state::<Settings>()
        .update::<Combos>(app, Partition::Combos, |c| {
            let r = c.change(account, op);
            let ok = r.is_ok();
            result = Some(r);
            ok
        });
    if let Err(e) = saved {
        eprintln!("combos: couldn't save: {e}");
    }
    result.unwrap_or(Err(ComboError::Missing))
}
