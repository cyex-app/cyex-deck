//! Export and import format (2.9) and share codes (X6).

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::macros::format_description;

use crate::Partition;
use crate::error::{Error, Result};

const FORMAT: &str = "cyex-settings";
const VERSION: u32 = 1;
const CODE_PREFIX: &str = "cyex1.";
/// V107-M0 Far above any real export; stops a huge file or pasted code from filling memory.
pub const MAX_IMPORT_BYTES: usize = 16 * 1024 * 1024;

/// Exported settings. A single-partition export is a bundle with one entry, so one
/// importer handles both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub version: u32,
    /// `yyyy/mm/dd hh:mm`, for the user's eyes only.
    pub exported: String,
    pub partitions: BTreeMap<String, Value>,
}

impl Bundle {
    pub(crate) fn new(partitions: BTreeMap<String, Value>, now: OffsetDateTime) -> Self {
        let exported = now
            .format(format_description!("[year]/[month]/[day] [hour]:[minute]"))
            .expect("date formats");
        Self {
            format: FORMAT.into(),
            version: VERSION,
            exported,
            partitions,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("bundle serializes")
    }

    pub fn from_json(text: &str) -> Result<Self> {
        if text.len() > MAX_IMPORT_BYTES {
            return Err(Error::TooBig);
        }
        let bundle: Bundle = serde_json::from_str(text).map_err(|_| Error::NotSettings)?;
        if bundle.format != FORMAT {
            return Err(Error::NotSettings);
        }
        if bundle.version > VERSION {
            return Err(Error::TooNew);
        }
        Ok(bundle)
    }

    pub fn to_share_code(&self) -> String {
        let compact = serde_json::to_vec(self).expect("bundle serializes");
        format!("{CODE_PREFIX}{}", URL_SAFE_NO_PAD.encode(compact))
    }

    pub fn from_share_code(code: &str) -> Result<Self> {
        // Base64 is 4/3 the size of what it carries.
        if code.len() > MAX_IMPORT_BYTES / 3 * 4 + 64 {
            return Err(Error::TooBig);
        }
        let body: String = code.split_whitespace().collect();
        let body = body.strip_prefix(CODE_PREFIX).ok_or(Error::BadCode)?;
        let bytes = URL_SAFE_NO_PAD.decode(body).map_err(|_| Error::BadCode)?;
        let text = String::from_utf8(bytes).map_err(|_| Error::BadCode)?;
        Self::from_json(&text).map_err(|e| match e {
            Error::TooNew => Error::TooNew,
            Error::TooBig => Error::TooBig,
            _ => Error::BadCode,
        })
    }
}

/// `cyex-general-2026-10-09.json` for one partition, `cyex-settings-2026-10-09.json` otherwise.
pub fn suggested_file_name(partitions: &[Partition], now: OffsetDateTime) -> String {
    let date = now
        .format(format_description!("[year]-[month]-[day]"))
        .expect("date formats");
    match partitions {
        [single] => format!("cyex-{}-{date}.json", single.key()),
        _ => format!("cyex-settings-{date}.json"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::day;
    use serde_json::json;
    use time::Time;

    fn sample() -> Bundle {
        let now = day(9).with_time(Time::from_hms(14, 3, 0).unwrap()).assume_utc();
        Bundle::new(
            BTreeMap::from([("general".into(), json!({"tooltips": false}))]),
            now,
        )
    }

    #[test]
    fn exported_stamp_uses_app_date_format() {
        assert_eq!(sample().exported, "2026/10/09 14:03");
    }

    #[test]
    fn json_round_trip() {
        let bundle = sample();
        assert_eq!(Bundle::from_json(&bundle.to_json()).unwrap(), bundle);
    }

    #[test]
    fn combos_travel_in_a_bundle() {
        // V1051-04
        assert!(Partition::ALL.contains(&Partition::Combos));
        let combos = json!({"accounts": {"1": [{"id": "a", "name": "gold", "paint": null, "badge": null}]}});
        let bundle = Bundle::new(BTreeMap::from([("combos".into(), combos.clone())]), day(9).with_time(Time::MIDNIGHT).assume_utc());
        let back = Bundle::from_json(&bundle.to_json()).unwrap();
        assert_eq!(back.partitions["combos"], combos);
    }

    #[test]
    fn share_code_round_trip_ignores_whitespace() {
        let bundle = sample();
        let code = bundle.to_share_code();
        assert!(code.starts_with(CODE_PREFIX));
        let wrapped = format!("  {}\n{}  ", &code[..10], &code[10..]);
        assert_eq!(Bundle::from_share_code(&wrapped).unwrap(), bundle);
    }

    #[test]
    fn rejects_foreign_and_newer_files() {
        assert!(matches!(Bundle::from_json("{\"a\":1}"), Err(Error::NotSettings)));
        assert!(matches!(Bundle::from_json("not json"), Err(Error::NotSettings)));
        let mut newer = sample();
        newer.version = VERSION + 1;
        assert!(matches!(Bundle::from_json(&newer.to_json()), Err(Error::TooNew)));
    }

    #[test]
    fn rejects_broken_codes() {
        let code = sample().to_share_code();
        assert!(matches!(Bundle::from_share_code("hello"), Err(Error::BadCode)));
        assert!(matches!(
            Bundle::from_share_code(&code[..code.len() - 6]),
            Err(Error::BadCode)
        ));
    }

    #[test]
    fn rejects_oversized_imports() {
        let big = " ".repeat(MAX_IMPORT_BYTES + 1);
        assert!(matches!(Bundle::from_json(&big), Err(Error::TooBig)));
        let code = format!("{CODE_PREFIX}{}", "A".repeat(MAX_IMPORT_BYTES / 3 * 4 + 64));
        assert!(matches!(Bundle::from_share_code(&code), Err(Error::TooBig)));
    }

    #[test]
    fn file_names() {
        let now = day(9).midnight().assume_utc();
        assert_eq!(
            suggested_file_name(&[Partition::Chat], now),
            "cyex-chat-2026-10-09.json"
        );
        assert_eq!(
            suggested_file_name(&Partition::ALL, now),
            "cyex-settings-2026-10-09.json"
        );
    }
}
