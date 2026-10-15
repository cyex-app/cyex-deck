//! V1051-01 How Windows' volume mixer shows the app: names and grouping params.

use cyex_core::app::NAME;

/// A set of sessions in the mixer: its name and grouping param (sessions with the same
/// group are one entry, also across processes; measured in both mixers).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    pub name: String,
    pub group: u128,
}

const GROUP_APP: u128 = 0x5f0c2a91_3b7e_4d6a_9c11_2e8f4b7a6d02;
const GROUP_STREAMS: u128 = 0x5f0c2a91_3b7e_4d6a_9c11_2e8f4b7a6d03;
const GROUP_SOUNDS: u128 = 0x5f0c2a91_3b7e_4d6a_9c11_2e8f4b7a6d04;

/// Streams' look (WebView2's sessions), then the app's own sounds' look.
pub fn looks(separate_streams: bool) -> (Look, Look) {
    if separate_streams {
        (
            Look {
                name: format!("{NAME} - streams"),
                group: GROUP_STREAMS,
            },
            Look {
                name: NAME.to_owned(),
                group: GROUP_SOUNDS,
            },
        )
    } else {
        let one = Look {
            name: NAME.to_owned(),
            group: GROUP_APP,
        };
        (one.clone(), one)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separate_streams_are_their_own_entry() {
        let (streams, sounds) = looks(true);
        assert_eq!(
            (streams.name.as_str(), sounds.name.as_str()),
            ("cyex deck - streams", "cyex deck")
        );
        assert_ne!(streams.group, sounds.group);
    }

    #[test]
    fn combined_is_one_entry() {
        let (streams, sounds) = looks(false);
        assert_eq!(streams, sounds);
        assert_eq!(streams.name, "cyex deck");
    }
}
