//! V107-Mk Smiley style: which art `:)` `:/` `:P` and friends get in your own messages
//! (Robot set 0, Glitch set 33, Monkey set 42). A Prime/Turbo setting, read and written
//! as twitch.tv's Settings › Prime page does. Twitch's set names aren't fixed here: each
//! offered set is matched by its emote ids. Research: `research/twitch-api.md`, "Chat identity".

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, EmoteKind, EmoteSet, TwitchEmote, run_at};
use crate::{Http, Result};

const READ: &str = "query SettingsPrimePage_PrimeEmotesSetPicker { currentUser { id primeSettings { id \
    primeEmoticons { currentSetName emoticonSets { id emotes { id text } } } } } }";

const WRITE: &str = "mutation SettingsPrimePage_PrimeEmotesSetPicker_UpdatePrimeSettings($input: UpdatePrimeSettingsInput!) { \
    updatePrimeSettings(input: $input) { primeSettings { id primeEmoticons { currentSetName } } } }";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SmileyStyle {
    #[default]
    Robot,
    Glitch,
    Monkey,
}

impl SmileyStyle {
    pub const ALL: [SmileyStyle; 3] = [SmileyStyle::Robot, SmileyStyle::Glitch, SmileyStyle::Monkey];

    /// Twitch's emote set with this style's art.
    pub fn set_id(self) -> &'static str {
        match self {
            SmileyStyle::Robot => "0",
            SmileyStyle::Glitch => "33",
            SmileyStyle::Monkey => "42",
        }
    }

    /// (code, emote id), the classic ids first.
    pub fn emotes(self) -> &'static [(&'static str, &'static str)] {
        match self {
            SmileyStyle::Robot => ROBOT,
            SmileyStyle::Glitch => GLITCH,
            SmileyStyle::Monkey => MONKEY,
        }
    }

    pub fn owns(self, emote_id: &str) -> bool {
        self.emotes().iter().any(|(_, id)| *id == emote_id)
    }

    /// The style whose art `emote_id` is.
    pub fn of(emote_id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.owns(emote_id))
    }

    /// The emote id `code` gets in this style.
    pub fn id_for(self, code: &str) -> Option<&'static str> {
        self.emotes().iter().find(|(c, _)| *c == code).map(|(_, id)| *id)
    }
}

/// One style Twitch offers this account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SmileyChoice {
    pub style: SmileyStyle,
    /// Twitch's name for the set, sent back to pick it.
    pub name: String,
    /// A few emote ids to show it with.
    pub preview: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Smilies {
    pub current: SmileyStyle,
    /// Empty or one: nothing to pick, so the app doesn't show it.
    pub choices: Vec<SmileyChoice>,
}

/// The account's smiley style and the styles it may pick from.
pub async fn smilies(http: &Http, auth: Auth<'_>, base: Option<&str>) -> Result<Smilies> {
    Ok(read(&run_at(http, auth, base, READ, json!({})).await?))
}

/// Picks the set Twitch calls `name` (a [`SmileyChoice::name`]). `Ok(false)`: Twitch ran
/// it but kept another set.
pub async fn set_smilies(http: &Http, auth: Auth<'_>, base: Option<&str>, user_id: &str, name: &str) -> Result<bool> {
    let vars = json!({ "input": { "primeEmoticonsSetName": name, "userID": user_id } });
    let v = run_at(http, auth, base, WRITE, vars).await?;
    let now = v.pointer("/updatePrimeSettings/primeSettings/primeEmoticons/currentSetName");
    Ok(now.and_then(Value::as_str) == Some(name))
}

pub(crate) fn read(data: &Value) -> Smilies {
    let prime = data.pointer("/currentUser/primeSettings/primeEmoticons");
    let current_name = prime.and_then(|p| p["currentSetName"].as_str()).unwrap_or_default();
    let mut choices: Vec<SmileyChoice> = Vec::new();
    for set in prime.and_then(|p| p["emoticonSets"].as_array()).into_iter().flatten() {
        let Some(name) = set["id"].as_str() else { continue };
        let ids: Vec<&str> = set["emotes"].as_array().into_iter().flatten().filter_map(|e| e["id"].as_str()).collect();
        let Some(style) = ids.iter().find_map(|id| SmileyStyle::of(id)) else { continue };
        if choices.iter().any(|c| c.style == style) {
            continue;
        }
        choices.push(SmileyChoice {
            style,
            name: name.to_owned(),
            preview: ids.iter().take(3).map(|s| (*s).to_owned()).collect(),
        });
    }
    let current = choices.iter().find(|c| c.name == current_name).map_or(SmileyStyle::Robot, |c| c.style);
    choices.sort_by_key(|c| SmileyStyle::ALL.iter().position(|s| *s == c.style));
    if choices.len() < 2 {
        choices.clear();
    }
    Smilies { current, choices }
}

/// Puts `style`'s smileys in the account's Twitch sets in place of the other styles', the
/// way twitch.tv draws your own `:)`. Sets left empty go; the style's art comes in when
/// Twitch's list doesn't have it. Running it again with another style undoes it.
pub fn apply(sets: &mut Vec<EmoteSet>, style: SmileyStyle) {
    let others: Vec<SmileyStyle> = SmileyStyle::ALL.into_iter().filter(|s| *s != style).collect();
    let smiley_sets = SmileyStyle::ALL.map(SmileyStyle::set_id);
    for set in sets.iter_mut().filter(|s| smiley_sets.contains(&s.id.as_str())) {
        set.emotes.retain(|e| !others.iter().any(|o| o.owns(&e.id)));
    }
    sets.retain(|s| !(smiley_sets.contains(&s.id.as_str()) && s.emotes.is_empty()));
    if sets.iter().any(|s| s.emotes.iter().any(|e| style.owns(&e.id))) {
        return;
    }
    let emotes = style.emotes().iter().map(|(code, id)| TwitchEmote {
        id: (*id).to_owned(),
        name: (*code).to_owned(),
        set_id: style.set_id().to_owned(),
        kind: EmoteKind::Global,
        animated: false,
        modifiers: Vec::new(),
    });
    match sets.iter_mut().find(|s| s.id == style.set_id()) {
        Some(set) => set.emotes.extend(emotes),
        None => sets.push(EmoteSet {
            id: style.set_id().to_owned(),
            owner: None,
            emotes: emotes.collect(),
            tier: None,
        }),
    }
}

// From `emoteSet(id:)` 0, 33 and 42 (2026/10/22, anonymous), classic ids first.
const ROBOT: &[(&str, &str)] = &[
    (":)", "1"), (":(", "2"), (":D", "3"), (">(", "4"), (":|", "5"), ("O_o", "6"), ("B)", "7"), (":O", "8"),
    ("<3", "9"), (":/", "10"), (";)", "11"), (":P", "12"), (";P", "13"), ("R)", "14"), (":-)", "555555557"),
    (":(", "555555558"), (":-(", "555555559"), (":D", "555555560"), (":-D", "555555561"), (">(", "555555562"),
    (":|", "555555563"), (":-|", "555555564"), (":z", "555555565"), (":-z", "555555566"), (":Z", "555555567"),
    (":-Z", "555555568"), ("O_o", "555555569"), ("O.o", "555555570"), ("O_O", "555555571"),
    ("O.O", "555555572"), ("o_O", "555555573"), ("o.O", "555555574"), ("o_o", "555555575"),
    ("o.o", "555555576"), ("B)", "555555577"), ("B-)", "555555578"), ("8-)", "555555579"),
    (":O", "555555580"), (":-O", "555555581"), (":o", "555555582"), (":-o", "555555583"), ("<3", "555555584"),
    (":/", "555555585"), (":-/", "555555586"), (":\\", "555555587"), (":-\\", "555555588"),
    (";)", "555555589"), (";-)", "555555590"), (":P", "555555591"), (":-P", "555555592"), (":p", "555555593"),
    (":-p", "555555594"), (";P", "555555595"), (";-P", "555555596"), (";p", "555555597"),
    (";-p", "555555598"), ("R)", "555555599"), ("R-)", "555555600"),
];
const GLITCH: &[(&str, &str)] = &[
    (">(", "432"), (":/", "433"), (":(", "434"), ("R)", "435"), (":O", "436"), ("O_o", "437"), (":P", "438"),
    (";)", "439"), (":)", "440"), ("B)", "441"), (";P", "442"), (":D", "443"), (":|", "444"), ("<3", "445"),
    (">(", "555555601"), (":/", "555555602"), (":-/", "555555603"), (":\\", "555555604"),
    (":-\\", "555555605"), (":(", "555555606"), (":-(", "555555607"), ("R)", "555555608"),
    ("R-)", "555555609"), (":O", "555555610"), (":-O", "555555611"), (":o", "555555612"),
    (":-o", "555555613"), ("O_o", "555555614"), ("O.o", "555555615"), ("O_O", "555555616"),
    ("O.O", "555555617"), ("o_O", "555555618"), ("o.O", "555555619"), ("o_o", "555555620"),
    ("o.o", "555555621"), (":P", "555555622"), (":-P", "555555623"), (":p", "555555624"),
    (":-p", "555555625"), (";)", "555555626"), (";-)", "555555627"), (":)", "555555628"),
    (":-)", "555555629"), ("B)", "555555630"), ("B-)", "555555631"), ("8-)", "555555632"),
    (";P", "555555633"), (";-P", "555555634"), (";p", "555555635"), (";-p", "555555636"), (":D", "555555637"),
    (":-D", "555555638"), (":|", "555555639"), (":-|", "555555640"), (":z", "555555641"),
    (":-z", "555555642"), (":Z", "555555643"), (":-Z", "555555644"), ("<3", "555555645"),
];
const MONKEY: &[(&str, &str)] = &[
    ("<3", "483"), ("R)", "484"), ("#/", "485"), (":>", "486"), ("<]", "487"), (":7", "488"), (":(", "489"),
    (":P", "490"), (";P", "491"), (":O", "492"), (":/", "493"), (":|", "494"), (":s", "495"), (":D", "496"),
    ("O_o", "497"), (">(", "498"), (":)", "499"), ("B)", "500"), (";)", "501"), ("R-)", "555555648"),
    ("#-/", "555555650"), ("#\\", "555555651"), ("#-\\", "555555652"), (":-7", "555555656"),
    (":L", "555555657"), (":-L", "555555658"), (":-(", "555555660"), (":-P", "555555662"),
    (":p", "555555663"), (":-p", "555555664"), (";-P", "555555666"), (";p", "555555667"),
    (";-p", "555555668"), (":-O", "555555670"), (":o", "555555671"), (":-o", "555555672"),
    (":-/", "555555674"), (":\\", "555555675"), (":-\\", "555555676"), (":-|", "555555678"),
    (":z", "555555679"), (":-z", "555555680"), (":Z", "555555681"), (":-Z", "555555682"), (":S", "555555683"),
    (":-S", "555555684"), (":s", "555555685"), (":-s", "555555686"), (":D", "555555687"),
    (":-D", "555555688"), ("O_o", "555555689"), ("O.o", "555555690"), ("O_O", "555555691"),
    ("O.O", "555555692"), ("o_O", "555555693"), ("o.O", "555555694"), ("o_o", "555555695"),
    ("o.o", "555555696"), ("B-)", "555555699"), (";-)", "555555701"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gql::dashboard::mock;

    fn emote(id: &str, name: &str, set: &str) -> TwitchEmote {
        TwitchEmote {
            id: id.into(),
            name: name.into(),
            set_id: set.into(),
            kind: EmoteKind::Global,
            animated: false,
            modifiers: Vec::new(),
        }
    }

    fn set(id: &str, emotes: Vec<TwitchEmote>) -> EmoteSet {
        EmoteSet {
            id: id.into(),
            owner: None,
            emotes,
            tier: None,
        }
    }

    fn answer(current: &str, sets: &[(&str, &[&str])]) -> Value {
        let sets: Vec<Value> = sets
            .iter()
            .map(|(id, ids)| json!({ "id": id, "emotes": ids.iter().map(|i| json!({ "id": i, "text": "x" })).collect::<Vec<_>>() }))
            .collect();
        json!({ "currentUser": { "id": "7", "primeSettings": { "id": "p", "primeEmoticons": { "currentSetName": current, "emoticonSets": sets } } } })
    }

    #[test]
    fn codes_map_per_style() {
        assert_eq!(SmileyStyle::Robot.id_for(":)"), Some("1"));
        assert_eq!(SmileyStyle::Glitch.id_for(":)"), Some("440"));
        assert_eq!(SmileyStyle::Monkey.id_for(":)"), Some("499"));
        assert_eq!(SmileyStyle::Glitch.id_for(":/"), Some("433"));
        assert_eq!(SmileyStyle::Monkey.id_for(":P"), Some("490"));
        assert_eq!(SmileyStyle::of("555555628"), Some(SmileyStyle::Glitch));
        assert_eq!(SmileyStyle::of("25"), None);
        // No id is in two styles.
        for s in SmileyStyle::ALL {
            for (_, id) in s.emotes() {
                assert_eq!(SmileyStyle::of(id), Some(s), "{id}");
            }
        }
    }

    #[test]
    fn styles_are_matched_by_their_emotes() {
        let got = read(&answer("turbo", &[("robots", &["1", "2"]), ("monkeys", &["499", "483"]), ("turbo", &["440", "433", "434", "435"])]));
        assert_eq!(got.current, SmileyStyle::Glitch);
        let order: Vec<_> = got.choices.iter().map(|c| (c.style, c.name.as_str())).collect();
        assert_eq!(order, [(SmileyStyle::Robot, "robots"), (SmileyStyle::Glitch, "turbo"), (SmileyStyle::Monkey, "monkeys")]);
        assert_eq!(got.choices[1].preview, ["440", "433", "434"]);
    }

    #[test]
    fn nothing_to_pick_hides_it() {
        assert_eq!(read(&answer("robots", &[("robots", &["1"])])), Smilies::default());
        assert_eq!(read(&json!({ "currentUser": { "id": "7", "primeSettings": null } })), Smilies::default());
        assert_eq!(read(&json!({ "currentUser": null })), Smilies::default());
        // A set with no smiley art isn't a style.
        let got = read(&answer("robots", &[("robots", &["1"]), ("odd", &["25"])]));
        assert!(got.choices.is_empty());
    }

    #[test]
    fn apply_swaps_the_art() {
        let mut sets = vec![set("0", vec![emote("25", "Kappa", "0"), emote("1", ":)", "0"), emote("10", ":/", "0")]), set("9", vec![emote("77", "Hi", "9")])];
        apply(&mut sets, SmileyStyle::Glitch);
        assert_eq!(sets.len(), 3);
        assert_eq!(sets[0].emotes.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["25"]);
        let glitch = &sets[2];
        assert_eq!(glitch.id, "33");
        // The classic id comes first, so a lookup by name finds it.
        assert_eq!(glitch.emotes.iter().find(|e| e.name == ":)").map(|e| e.id.as_str()), Some("440"));

        // Twitch's list already holding the style: nothing added, the others go.
        let mut sets = vec![set("0", vec![emote("1", ":)", "0")]), set("42", vec![emote("499", ":)", "42")]), set("33", vec![emote("440", ":)", "33")])];
        apply(&mut sets, SmileyStyle::Monkey);
        assert_eq!(sets.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["42"]);

        // Robot keeps set 0 as it is.
        let mut sets = vec![set("0", vec![emote("1", ":)", "0")])];
        apply(&mut sets, SmileyStyle::Robot);
        assert_eq!(sets[0].emotes.len(), 1);

        // And comes back into set 0 after another style took its place.
        let mut sets = vec![set("0", vec![emote("25", "Kappa", "0"), emote("1", ":)", "0")])];
        apply(&mut sets, SmileyStyle::Glitch);
        apply(&mut sets, SmileyStyle::Robot);
        assert_eq!(sets.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["0"]);
        assert_eq!(sets[0].emotes.iter().find(|e| e.name == ":)").map(|e| e.id.as_str()), Some("1"));
    }

    #[tokio::test]
    async fn read_and_write_send_twitchs_operations() {
        let m = mock::serve([
            json!({ "data": answer("robots", &[("robots", &["1"]), ("turbo", &["440"])]) }).to_string(),
            r#"{"data":{"updatePrimeSettings":{"primeSettings":{"id":"p","primeEmoticons":{"currentSetName":"turbo"}}}}}"#.into(),
            r#"{"data":{"updatePrimeSettings":{"primeSettings":{"id":"p","primeEmoticons":{"currentSetName":"robots"}}}}}"#.into(),
        ]);
        let base = Some(m.base.as_str());
        let got = smilies(&m.http, mock::AUTH, base).await.unwrap();
        assert_eq!(got.choices.len(), 2);
        assert!(set_smilies(&m.http, mock::AUTH, base, "7", "turbo").await.unwrap());
        assert!(!set_smilies(&m.http, mock::AUTH, base, "7", "monkeys").await.unwrap());
        let sent = m.sent();
        assert!(mock::query(&sent[0]).contains("primeEmoticons { currentSetName emoticonSets"));
        assert!(mock::query(&sent[1]).contains("updatePrimeSettings(input: $input)"));
        assert_eq!(sent[1]["variables"], json!({ "input": { "primeEmoticonsSetName": "turbo", "userID": "7" } }));
    }
}
