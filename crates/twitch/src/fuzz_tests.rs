//! V110-F89 Decoders on generated JSON: a Twitch (or mock, or proxy) answer of any shape is a
//! decode error or a value, never a panic. The generator draws keys from the names the
//! decoders themselves read (harvested from this crate's sources), so paths chain deeper
//! than random keys would.

use serde_json::{Value, json};

use crate::fuzz::{Rng, byte_prefixes};
use crate::fuzz_json::{dictionary, tree, tweak};
use crate::gql::dashboard::content::{clips, videos};
use crate::gql::dashboard::rewards;
use crate::gql::{drops, hypetrain, milestones, miner, modlogs, notifications, polls, predictions, smilies, standing, whispers};
use crate::{clock, embed, uploader};

const DROPS_INVENTORY: &str = r#"{"currentUser":{"inventory":{"dropCampaignsInProgress":[{"id":"c","name":"Fall drops",
    "game":{"id":"1","slug":"rust","displayName":"Rust"},"timeBasedDrops":[
    {"id":"d1","name":"Drop 1","requiredMinutesWatched":60,"benefitEdges":[{"benefit":{"id":"b1","name":"Hat","imageAssetURL":"x.png","distributionType":"DIRECT_ENTITLEMENT"}}],
     "self":{"hasPreconditionsMet":true,"currentMinutesWatched":60,"dropInstanceID":"i1","isClaimed":false}},
    {"id":"d2","name":"Drop 2","requiredMinutesWatched":120,"benefitEdges":[],
     "self":{"hasPreconditionsMet":true,"currentMinutesWatched":61,"dropInstanceID":null,"isClaimed":false}}]}],
     "gameEventDrops":[{"id":"b9"},{"id":"b1"}]}}}"#;

const DROPS_DETAILS: &str = r#"{"currentUser":{"dropCampaign":{"id":"c1","name":"Event","status":"ACTIVE",
    "startAt":"2026-11-01T10:00:00Z","endAt":"2026-11-30T10:00:00.000Z","game":{"id":"1","slug":"s","displayName":"G"},
    "allow":{"isEnabled":true,"channels":[{"id":"1","name":"a"}]},"timeBasedDrops":[{"id":"d","name":"n","requiredMinutesWatched":30,
    "startAt":"2026-11-01T10:00:00Z","endAt":"2026-11-30T10:00:00Z","benefitEdges":[{"benefit":{"id":"b","name":"B","imageAssetURL":"u","distributionType":"BADGE"}}]}]}}}"#;

const DROPS_DASH: &str = r#"{"currentUser":{"dropCampaigns":[{"id":"a","status":"ACTIVE"},{"id":"b","status":"EXPIRED"}]}}"#;

const HERMES_POLL: &str = r#"{"type":"POLL_UPDATE","data":{"poll":{"poll_id":"p","title":"t","status":"ACTIVE","started_at":"2026-11-10T01:00:00Z","ended_at":null,"duration_seconds":60,"settings":{"channel_points_votes":{"is_enabled":true,"cost":10}},"choices":[{"choice_id":"c","title":"a","votes":{"total":5,"bits":0,"channel_points":5,"base":0},"tokens":{"bits":0,"channel_points":0},"total_voters":3}],"votes":{"total":5},"total_voters":3}}}"#;

const HERMES_PRED: &str = r#"{"type":"event-updated","data":{"timestamp":"2026-11-10T01:00:00Z","event":{"id":"e","title":"t","status":"ACTIVE","created_at":"2026-11-10T01:00:00Z","prediction_window_seconds":60,"outcomes":[{"id":"o","title":"a","color":"BLUE","total_points":10,"total_users":2,"top_predictors":[{"user_display_name":"x","points":5}]}]}}}"#;

const OEMBED: &str = r#"<html><head><meta property="og:title" content="T"><meta property="og:image" content="https://a.b/c.png"><meta name="twitter:card" content="summary"><title>x</title></head></html>"#;

fn run_all(v: &Value) {
    let _ = drops::parse_channel(v.clone());
    let _ = drops::parse_dashboard(v.clone());
    let _ = drops::campaign_from_details(v.clone());
    let _ = drops::parse_inventory(v.clone());
    let _ = rewards::read_rewards(v);
    let _ = rewards::read_saved(v.clone());
    let _ = rewards::read_queue(v);
    let _ = clips::read_list(v.clone());
    let _ = clips::read_rename(v.clone(), "title");
    let _ = clips::read_delete(v.clone());
    let _ = clips::matching(v.clone(), "a");
    let _ = videos::read_list(v.clone());
    let _ = videos::read_update(v.clone());
    let _ = videos::read_delete(v.clone());
    let _ = videos::confirmed(v, &["1".to_owned()], true);
    let _ = miner::streak_of(v);
    let _ = hypetrain::event_of(v);
    let _ = hypetrain::user_event_of(v);
    let _ = milestones::update_of(v);
    let _ = notifications::from_hermes(v);
    let _ = whispers::from_hermes(v);
    let _ = polls::Poll::from_hermes(v, vec!["c".into()]);
    let _ = predictions::Prediction::from_hermes(v, None);
    let _ = standing::change_of(v);
    let _ = smilies::read(v);
    let _ = modlogs::ban_of(v);
}

#[test]
fn decoders_never_panic_on_generated_json() {
    let dict = dictionary(env!("CARGO_MANIFEST_DIR"));
    assert!(dict.len() > 300, "dictionary harvest failed ({})", dict.len());
    let mut r = Rng::new(0x7771);
    let start = std::time::Instant::now();
    for _ in 0..4000 {
        let v = tree(&mut r, &dict, 0);
        run_all(&v);
    }
    assert!(start.elapsed().as_secs() < 30, "{:?}", start.elapsed());
}

#[test]
fn decoders_never_panic_on_mutated_real_shapes() {
    let dict = dictionary(env!("CARGO_MANIFEST_DIR"));
    let mut r = Rng::new(0x7772);
    let seeds: Vec<Value> = [DROPS_INVENTORY, DROPS_DETAILS, DROPS_DASH, HERMES_POLL, HERMES_PRED]
        .iter()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    // The seeds themselves still decode.
    assert!(drops::parse_inventory(seeds[0].clone()).is_ok());
    assert!(drops::campaign_from_details(seeds[1].clone()).is_some());
    assert!(drops::parse_dashboard(seeds[2].clone()).is_ok());
    assert!(polls::Poll::from_hermes(&seeds[3], vec![]).is_some());
    assert!(predictions::Prediction::from_hermes(&seeds[4], None).is_some());
    for _ in 0..1200 {
        for s in &seeds {
            let mut v = s.clone();
            for _ in 0..1 + r.below(3) {
                tweak(&mut r, &dict, &mut v);
            }
            run_all(&v);
            // Also with the answer wrapped as `data`.
            run_all(&json!({ "data": v }));
        }
    }
}

#[test]
fn truncated_and_mutated_text_bodies_never_panic() {
    let mut r = Rng::new(0x7773);
    for s in [OEMBED, DROPS_INVENTORY, HERMES_POLL] {
        for p in byte_prefixes(s) {
            let _ = embed::card(&p);
            let _ = embed::decide(Some(200), &p);
            if let Ok(v) = serde_json::from_str::<Value>(&p) {
                run_all(&v);
            }
        }
        for _ in 0..400 {
            let m = r.mutate(s);
            let _ = embed::card(&m);
            for status in [None, Some(0), Some(200), Some(301), Some(404), Some(500), Some(65535)] {
                let _ = embed::decide(status, &m);
            }
            if let Ok(v) = serde_json::from_str::<Value>(&m) {
                run_all(&v);
            }
        }
    }
    for _ in 0..2000 {
        let t = r.text(60);
        let _ = embed::card(&t);
        let _ = embed::is_img(&t);
        let _ = clock::parse_rfc3339(&t);
        let _ = uploader::headers(&t);
        let _ = uploader::fill(&t, t.as_bytes());
        let _ = uploader::error_text(r.below(700) as u16, t.as_bytes());
        let _ = uploader::multipart(&t, &t, &t, t.as_bytes());
    }
}

#[test]
fn rfc3339_parses_what_it_prints_and_nothing_wild() {
    let mut r = Rng::new(0x7774);
    let samples = ["2026-11-10T01:02:03Z", "2026-11-10T01:02:03.999999999Z", "2026-11-10T01:02:03+01:00", "2026-11-10T01:02:03-23:59", "0001-01-01T00:00:00Z", "9999-12-31T23:59:59Z", "2026-02-30T00:00:00Z", "2026-11-10 01:02:03", "1970-01-01T00:00:00Z"];
    for s in samples {
        for p in byte_prefixes(s) {
            let _ = clock::parse_rfc3339(&p);
        }
        for _ in 0..300 {
            let _ = clock::parse_rfc3339(&r.mutate(s));
        }
    }
    for secs in [0i64, 1, -1, 86_399, 86_400, 1_700_000_000, 253_402_300_799, -62_135_596_800, -62_167_219_200] {
        let text = clock::rfc3339(secs);
        assert_eq!(clock::parse_rfc3339(&text), Some(secs), "{secs} -> {text}");
    }
    // Printing extremes must not panic either.
    for secs in [i64::MAX, i64::MIN, i64::MAX - 1, 1 << 60, -(1 << 60)] {
        let _ = clock::rfc3339(secs);
    }
}
