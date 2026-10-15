use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// V1051-01 Settings → Sound: the app in Windows' volume mixer and where its sound plays.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Sound {
    /// Streams get their own entry ("cyex deck - streams") next to "cyex deck".
    pub separate_streams: bool,
    /// Where streams, VODs, clips and stories play. None follows Windows.
    pub streams_device: Option<OutputDevice>,
    /// Where mentions, highlights, live alerts and whispers play. None follows Windows.
    pub sounds_device: Option<OutputDevice>,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            separate_streams: true,
            streams_device: None,
            sounds_device: None,
        }
    }
}

/// An output device as Windows knows it: its endpoint id and name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
}

const MAX_ID_CHARS: usize = 300;
const MAX_NAME_CHARS: usize = 200;

impl Sound {
    pub(crate) fn sanitize(&mut self) {
        for d in [&mut self.streams_device, &mut self.sounds_device] {
            if let Some(dev) = d {
                dev.id = dev.id.trim().chars().take(MAX_ID_CHARS).collect();
                dev.name = dev.name.trim().chars().take(MAX_NAME_CHARS).collect();
                if dev.id.is_empty() {
                    *d = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_separate_and_follow_windows_by_default() {
        let s: Sound = serde_json::from_str("{}").unwrap();
        assert!(s.separate_streams && s.streams_device.is_none() && s.sounds_device.is_none());
    }

    #[test]
    fn a_device_without_id_follows_windows() {
        let mut s: Sound = serde_json::from_str(
            r#"{"streamsDevice":{"id":"  ","name":"x"},"soundsDevice":{"id":" a ","name":" Music "}}"#,
        )
        .unwrap();
        s.sanitize();
        assert_eq!(s.streams_device, None);
        assert_eq!(
            s.sounds_device,
            Some(OutputDevice {
                id: "a".into(),
                name: "Music".into()
            })
        );
    }
}
