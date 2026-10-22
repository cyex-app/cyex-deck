use super::*;
use crate::schema::{Appearance, Chat, General};
use crate::testutil::{FakeClock, TempDir, day};
use serde_json::json;

fn open(dir: &Path, clock: &Arc<FakeClock>) -> (Store, LoadReport) {
    Store::open(dir, clock.clone()).unwrap()
}

#[test]
fn starts_with_defaults_and_writes_nothing() {
    let tmp = TempDir::new("store-defaults");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (store, report) = open(tmp.path(), &clock);
    assert!(report.reset.is_empty());
    assert_eq!(store.read::<General>(Partition::General), General::default());
    assert!(!tmp.path().join("general.json").exists());
}

#[test]
fn mark_dirty_writes_at_flush() {
    let tmp = TempDir::new("store-mark-dirty");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    store.mark_dirty(Partition::Chat);
    store.flush().unwrap();
    assert!(tmp.path().join("chat.json").exists());
}

/// V107-A3 One file that can't be written keeps only itself dirty.
#[test]
fn a_failed_write_keeps_that_part_dirty_and_saves_the_rest() {
    let tmp = TempDir::new("store-flush-partial");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    for p in [Partition::Chat, Partition::General, Partition::Tabs] {
        store.mark_dirty(p);
    }
    let blocked = tmp.path().join(Partition::General.file_name());
    fs::create_dir(&blocked).unwrap();
    assert!(store.flush().is_err());
    assert!(tmp.path().join(Partition::Chat.file_name()).is_file());
    assert!(tmp.path().join(Partition::Tabs.file_name()).is_file());
    assert!(store.is_dirty());
    fs::remove_dir(&blocked).unwrap();
    store.flush().unwrap();
    assert!(blocked.is_file());
    assert!(!store.is_dirty());
}

/// V107-A3 A backup that can't be taken doesn't stop the save.
#[test]
fn a_backup_that_fails_does_not_block_saving() {
    let tmp = TempDir::new("store-flush-backup");
    let clock = Arc::new(FakeClock::at(day(9)));
    // A file on disk, so the day's backup has something to copy.
    fs::write(tmp.path().join(Partition::General.file_name()), "{}").unwrap();
    let (mut store, _) = open(tmp.path(), &clock);
    let backups = tmp.path().join("backups");
    if backups.is_dir() {
        fs::remove_dir_all(&backups).unwrap();
    }
    fs::write(&backups, "not a folder").unwrap();
    store.mark_dirty(Partition::Chat);
    assert!(store.flush().is_err());
    assert!(tmp.path().join(Partition::Chat.file_name()).is_file());
    assert!(!store.is_dirty());
}

#[test]
fn normalize_changes_are_written_once() {
    // R19 presetsMoved: false runs the move; the result reaches disk at the next flush.
    let tmp = TempDir::new("store-normalize-dirty");
    fs::write(tmp.path().join("appearance.json"), r#"{"presetsMoved": false}"#).unwrap();
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    assert!(store.is_dirty());
    store.flush().unwrap();
    let saved: Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join("appearance.json")).unwrap()).unwrap();
    assert_eq!(saved["presetsMoved"], true);
    let (again, _) = open(tmp.path(), &clock);
    assert!(!again.is_dirty());
}

#[test]
fn saved_partitions_are_not_dirty_on_the_next_start() {
    // R19 STOP check: normalize is stable, so a saved folder writes nothing at start.
    let tmp = TempDir::new("store-normalize-stable");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    for p in Partition::ALL {
        store.mark_dirty(p);
    }
    store.flush().unwrap();
    let (again, _) = open(tmp.path(), &clock);
    assert!(!again.is_dirty(), "dirty: {:?}", again.dirty);
}

#[test]
fn patch_flush_and_reload() {
    let tmp = TempDir::new("store-reload");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    store
        .patch(Partition::General, &json!({"chatFontSize": 16}))
        .unwrap();
    assert!(store.is_dirty());
    store.flush().unwrap();
    assert!(!store.is_dirty());

    let (reloaded, _) = open(tmp.path(), &clock);
    assert_eq!(reloaded.read::<General>(Partition::General).chat_font_size, 16);
}

#[test]
fn unchanged_values_are_not_marked_dirty() {
    let tmp = TempDir::new("store-clean");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    store
        .patch(Partition::General, &json!({"tooltips": true}))
        .unwrap();
    assert!(!store.is_dirty());
}

#[test]
fn null_resets_a_field_and_unknown_fields_are_dropped() {
    let tmp = TempDir::new("store-null");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    store
        .patch(Partition::General, &json!({"chatFontSize": 20, "bogus": 1}))
        .unwrap();
    assert!(store.get(Partition::General).get("bogus").is_none());
    store
        .patch(Partition::General, &json!({"chatFontSize": null}))
        .unwrap();
    assert_eq!(store.read::<General>(Partition::General).chat_font_size, 13);
}

#[test]
fn wrong_types_are_rejected_and_nothing_changes() {
    let tmp = TempDir::new("store-types");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    assert!(
        store
            .patch(Partition::General, &json!({"chatFontSize": "big"}))
            .is_err()
    );
    assert_eq!(store.read::<General>(Partition::General), General::default());
}

#[test]
fn sanitizes_values() {
    let tmp = TempDir::new("store-sanitize");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    store
        .patch(Partition::Appearance, &json!({"accent": "nope"}))
        .unwrap();
    store
        .patch(Partition::General, &json!({"chatFontSize": 200}))
        .unwrap();
    store
        .patch(
            Partition::Chat,
            &json!({"timeoutPresets": [600, 0, 60, 600, 99_999_999]}),
        )
        .unwrap();
    assert_eq!(store.read::<Appearance>(Partition::Appearance).accent, "#145aff");
    assert_eq!(store.read::<General>(Partition::General).chat_font_size, 32);
    assert_eq!(store.read::<Chat>(Partition::Chat).timeout_presets, vec![60, 600]);
}

#[test]
fn broken_file_is_kept_aside_and_reported() {
    let tmp = TempDir::new("store-broken");
    fs::write(tmp.path().join("chat.json"), "{ not json").unwrap();
    let clock = Arc::new(FakeClock::at(day(9)));
    let (store, report) = open(tmp.path(), &clock);
    assert_eq!(report.reset, vec![Partition::Chat]);
    let off = Chat { share_presence: false, ..Chat::default() };
    assert_eq!(store.read::<Chat>(Partition::Chat), off);
    assert!(!tmp.path().join("chat.json").exists());
    assert!(tmp.path().join("chat.broken-20261009-120000.json").exists());
}

#[test]
fn broken_chat_file_keeps_presence_off_across_starts() {
    // V107-R11 Defaults for the rest, presence off, and written so the next start agrees.
    for text in ["{ not json", "[1, 2]", "\"x\"", ""] {
        let tmp = TempDir::new("store-broken-presence");
        fs::write(tmp.path().join("chat.json"), text).unwrap();
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, report) = open(tmp.path(), &clock);
        assert_eq!(report.reset, vec![Partition::Chat], "{text}");
        let chat = store.read::<Chat>(Partition::Chat);
        assert!(!chat.share_presence, "{text}");
        assert_eq!(chat.timestamps, Chat::default().timestamps);
        store.flush().unwrap();
        let (again, report) = open(tmp.path(), &clock);
        assert!(report.reset.is_empty());
        assert!(!again.read::<Chat>(Partition::Chat).share_presence, "{text}");
    }
}

#[test]
fn unreadable_chat_file_keeps_presence_off() {
    // V107-R11 Not UTF-8: read fails before JSON parsing.
    let tmp = TempDir::new("store-unreadable-presence");
    fs::write(tmp.path().join("chat.json"), [0xff, 0xfe, 0x00, 0x7b]).unwrap();
    let clock = Arc::new(FakeClock::at(day(9)));
    let (store, report) = open(tmp.path(), &clock);
    assert_eq!(report.reset, vec![Partition::Chat]);
    assert!(!store.read::<Chat>(Partition::Chat).share_presence);
}

#[test]
fn salvaged_chat_file_keeps_its_own_presence() {
    // V107-R11 Only fully broken files are forced off; a readable choice stays.
    let tmp = TempDir::new("store-salvage-presence-on");
    fs::write(
        tmp.path().join("chat.json"),
        r#"{"timestamps": "yes", "presenceOnOnce": true, "sharePresence": true}"#,
    )
    .unwrap();
    let clock = Arc::new(FakeClock::at(day(9)));
    let (store, _) = open(tmp.path(), &clock);
    assert!(store.read::<Chat>(Partition::Chat).share_presence);

    let tmp = TempDir::new("store-salvage-presence-bad");
    fs::write(
        tmp.path().join("chat.json"),
        r#"{"sharePresence": "yes", "users": {"1": {"nickname": "bob", "note": ""}}}"#,
    )
    .unwrap();
    let (store, _) = open(tmp.path(), &clock);
    let chat = store.read::<Chat>(Partition::Chat);
    assert!(!chat.share_presence);
    assert_eq!(chat.users["1"].nickname, "bob");
}

#[test]
fn missing_chat_file_still_turns_presence_on() {
    // V107-R11 A first start is not a broken file.
    let tmp = TempDir::new("store-fresh-presence");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (store, _) = open(tmp.path(), &clock);
    assert!(store.read::<Chat>(Partition::Chat).share_presence);
}

#[test]
fn one_wrong_value_keeps_the_rest_of_the_file() {
    // t5 Only the bad fields fall back; the file is still kept aside and reported.
    let tmp = TempDir::new("store-salvage");
    fs::write(
        tmp.path().join("chat.json"),
        r#"{"timestamps": "yes", "historyLines": -5, "presenceOnOnce": true, "sharePresence": false, "users": {"1": {"nickname": "bob", "note": ""}}}"#,
    )
    .unwrap();
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, report) = open(tmp.path(), &clock);
    assert_eq!(report.reset, vec![Partition::Chat]);
    let chat = store.read::<Chat>(Partition::Chat);
    assert!(!chat.share_presence);
    assert_eq!(chat.users["1"].nickname, "bob");
    assert_eq!(chat.timestamps, Chat::default().timestamps);
    assert_eq!(chat.history_lines, Chat::default().history_lines);
    assert!(tmp.path().join("chat.broken-20261009-120000.json").exists());
    store.flush().unwrap();
    let (again, report) = open(tmp.path(), &clock);
    assert!(report.reset.is_empty());
    assert!(!again.read::<Chat>(Partition::Chat).share_presence);
}

#[test]
fn import_is_all_or_nothing() {
    let tmp = TempDir::new("store-import");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    let bad = Bundle::from_json(
        &json!({"format": "cyex-settings", "version": 1, "exported": "",
                "partitions": {"general": {"tooltips": false}, "nope": {}}})
        .to_string(),
    )
    .unwrap();
    assert!(matches!(store.import(bad), Err(Error::UnknownPartition(k)) if k == "nope"));
    assert!(store.read::<General>(Partition::General).tooltips);
}

#[test]
fn accounts_never_leave_the_machine() {
    let tmp = TempDir::new("store-local");
    let (mut store, _) = open(tmp.path(), &Arc::new(FakeClock::at(day(9))));
    store
        .patch(
            Partition::Accounts,
            &json!({"list": [{"userId": "1", "login": "a"}]}),
        )
        .unwrap();
    let bundle = store.export(&[Partition::Accounts, Partition::General]);
    assert_eq!(bundle.partitions.keys().collect::<Vec<_>>(), ["general"]);
    let foreign = Bundle::from_json(
        &json!({"format": "cyex-settings", "version": 1, "exported": "",
            "partitions": {"accounts": {"list": []}}})
        .to_string(),
    )
    .unwrap();
    assert!(store.import(foreign).unwrap().is_empty());
    assert_eq!(store.get(Partition::Accounts)["list"][0]["login"], "a");
}

#[test]
fn export_import_round_trip_between_stores() {
    let a_dir = TempDir::new("store-export-a");
    let b_dir = TempDir::new("store-export-b");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut a, _) = open(a_dir.path(), &clock);
    a.patch(
        Partition::Appearance,
        &json!({"theme": "light", "accent": "#ff0000"}),
    )
    .unwrap();
    a.patch(Partition::General, &json!({"tooltips": false})).unwrap();

    let code = a.export(&[Partition::Appearance]).to_share_code();
    let (mut b, _) = open(b_dir.path(), &clock);
    let imported = b.import(Bundle::from_share_code(&code).unwrap()).unwrap();
    assert_eq!(imported, vec![Partition::Appearance]);
    assert_eq!(b.get(Partition::Appearance), a.get(Partition::Appearance));
    assert!(
        b.read::<General>(Partition::General).tooltips,
        "other partitions untouched"
    );
}

/// V107-N5c A share code or file never changes this PC's presence choice.
#[test]
fn an_import_keeps_the_presence_choice_of_this_pc() {
    let tmp = TempDir::new("store-import-presence");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    store.patch(Partition::Chat, &json!({"sharePresence": false, "presenceOnOnce": true})).unwrap();
    let incoming = Bundle::from_json(
        &json!({"format": "cyex-settings", "version": 1, "exported": "",
            "partitions": {"chat": {"sharePresence": true, "presenceOnOnce": false, "fontSize": 17}}})
        .to_string(),
    )
    .unwrap();
    store.import(incoming).unwrap();
    let chat = store.get(Partition::Chat);
    assert_eq!(chat["sharePresence"], false);
    assert_eq!(chat["presenceOnOnce"], true);
}

#[test]
fn backup_rotation_across_days_and_restore() {
    let tmp = TempDir::new("store-rotation");
    let clock = Arc::new(FakeClock::at(day(1)));
    let (mut store, _) = open(tmp.path(), &clock);

    for size in 10..=15u8 {
        store
            .patch(Partition::General, &json!({"chatFontSize": size}))
            .unwrap();
        store.flush().unwrap();
        clock.advance_days(1);
    }
    // Day 1 had no files yet; days 2-6 were snapshotted; the newest three remain.
    assert_eq!(store.backups().unwrap(), vec![day(6), day(5), day(4)]);

    // Day 4's snapshot holds the state saved on day 3.
    store.restore_backup(day(4)).unwrap();
    assert_eq!(store.read::<General>(Partition::General).chat_font_size, 12);
    assert!(matches!(store.restore_backup(day(1)), Err(Error::NoBackup(_))));
}

#[test]
fn snapshot_before_restore_holds_unsaved_changes_and_restores() {
    let tmp = TempDir::new("store-snapshot");
    let clock = Arc::new(FakeClock::at(day(1)));
    let (mut store, _) = open(tmp.path(), &clock);
    store.patch(Partition::General, &json!({"chatFontSize": 11})).unwrap();
    let name = store.snapshot_now().unwrap().unwrap();
    assert_eq!(store.snapshots().unwrap(), vec![name.clone()]);
    assert!(store.backups().unwrap().iter().all(|d| *d == day(1)), "daily list ignores it");

    store.patch(Partition::General, &json!({"chatFontSize": 20})).unwrap();
    store.restore_snapshot(&name).unwrap();
    assert_eq!(store.read::<General>(Partition::General).chat_font_size, 11);
    assert!(matches!(store.restore_snapshot("../x"), Err(Error::NoBackup(_))));
}
/// V107-N10b A share code can't pick where the OBS status files go, or turn them on.
#[test]
fn an_import_keeps_the_obs_files_of_this_pc() {
    let tmp = TempDir::new("store-import-obs");
    let clock = Arc::new(FakeClock::at(day(9)));
    let (mut store, _) = open(tmp.path(), &clock);
    let incoming = Bundle::from_json(
        &json!({"format": "cyex-settings", "version": 1, "exported": "",
            "partitions": {"streammanager": {"obs": {"enabled": true, "folder": "C:\\Windows"}, "chat": {"share": 0.5}}}})
        .to_string(),
    )
    .unwrap();
    store.import(incoming).unwrap();
    let sm = store.get(Partition::StreamManager);
    assert_eq!(sm["obs"], json!({"enabled": false, "folder": ""}));
    assert_eq!(sm["chat"]["share"], 0.5);
}

/// V107-N10a A file cut short comes back whole from the newest backup that reads; the day's
/// backup never takes the broken copy.
#[test]
fn a_file_cut_short_comes_back_from_the_backup() {
    let tmp = TempDir::new("store-recover");
    let clock = Arc::new(FakeClock::at(day(8)));
    let (mut store, _) = open(tmp.path(), &clock);
    store.patch(Partition::General, &json!({"chatFontSize": 14})).unwrap();
    store.flush().unwrap();
    clock.advance_days(1);
    drop(open(tmp.path(), &clock)); // day 9's backup holds the good file
    fs::write(tmp.path().join("general.json"), "{\"chatFontS").unwrap();
    clock.advance_days(1);
    let (mut store, report) = open(tmp.path(), &clock);
    assert_eq!(report.restored, vec![(Partition::General, day(9))]);
    assert!(report.reset.is_empty() && report.io_errors.is_empty());
    assert_eq!(store.read::<General>(Partition::General).chat_font_size, 14);
    assert!(tmp.path().join("general.broken-20261010-120000.json").exists());
    assert!(!tmp.path().join("backups/2026-10-10/general.json").exists());
    store.flush().unwrap();
    let (_, report) = open(tmp.path(), &clock);
    assert!(report.restored.is_empty() && report.reset.is_empty());
}

#[test]
fn no_readable_backup_still_means_defaults() {
    let tmp = TempDir::new("store-recover-none");
    fs::write(tmp.path().join("general.json"), "").unwrap();
    let clock = Arc::new(FakeClock::at(day(9)));
    let (store, report) = open(tmp.path(), &clock);
    assert_eq!(report.reset, vec![Partition::General]);
    assert!(report.restored.is_empty());
    assert_eq!(store.read::<General>(Partition::General), General::default());
}

/// V107-N10a A backup that can't be written (here `backups` is a file) doesn't stop the start.
#[test]
fn a_failed_backup_at_start_is_reported_not_fatal() {
    let tmp = TempDir::new("store-backup-blocked");
    fs::write(tmp.path().join("general.json"), r#"{"chatFontSize": 15}"#).unwrap();
    fs::write(tmp.path().join("backups"), "not a folder").unwrap();
    let clock = Arc::new(FakeClock::at(day(9)));
    let (store, report) = Store::open(tmp.path(), clock).expect("starts anyway");
    assert_eq!(report.io_errors.len(), 1);
    assert_eq!(store.read::<General>(Partition::General).chat_font_size, 15);
}

// V106-33 The old mentions fold into Highlights.
mod merge {
    use super::*;
    use crate::schema::Highlights;

    const OLD_HIGHLIGHTS: &str = r##"{"mention": {"enabled": true, "color": "#ff0000ff"}, "lookalikes": true,
        "phrases": [{"pattern": "hello"}, {"pattern": "a+", "regex": true}], "users": [{"login": "bob"}]}"##;
    const OLD_MENTIONS: &str = r##"{"me": {"sound": false, "color": "#00ff00"},
        "list": [{"pattern": "cyex", "flavor": "python", "regex": true}, {"pattern": "hello"}],
        "global": true, "globalList": [{"pattern": "deck", "soundFile": "C:\\s\\g.wav"}]}"##;

    fn write(dir: &Path, name: &str, text: &str) {
        fs::write(dir.join(name), text).unwrap();
    }

    fn highlights(store: &Store) -> Highlights {
        store.read(Partition::Highlights)
    }

    fn patterns(list: &[crate::schema::PhraseRule]) -> Vec<&str> {
        list.iter().map(|p| p.pattern.as_str()).collect()
    }

    fn bundle(partitions: Value) -> Bundle {
        Bundle::from_json(
            &json!({"format": "cyex-settings", "version": 1, "exported": "", "partitions": partitions})
                .to_string(),
        )
        .unwrap()
    }

    fn v105(dir: &Path) {
        write(dir, "highlights.json", OLD_HIGHLIGHTS);
        write(dir, "mentions.json", OLD_MENTIONS);
    }

    fn file(dir: &Path, name: &str) -> String {
        fs::read_to_string(dir.join(name)).unwrap()
    }

    #[test]
    fn fresh_user_merges_nothing() {
        let tmp = TempDir::new("merge-fresh");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, report) = open(tmp.path(), &clock);
        assert!(report.merged.is_none() && !report.old_mentions_unreadable);
        let h = highlights(&store);
        assert!(h.me.alert.sound && !h.replies.alert.sound);
        assert!(h.me.alert.show && h.replies.alert.show);
        assert!(h.mentions_merged && h.duplicates_asked);
        store.flush().unwrap();
        assert!(!tmp.path().join("highlights.json").exists(), "nothing written");
    }

    #[test]
    fn v105_folder_is_merged_once() {
        let tmp = TempDir::new("merge-v105");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, report) = open(tmp.path(), &clock);
        let r = report.merged.unwrap();
        assert_eq!((r.from_mentions, r.from_highlights, r.global, r.duplicates, r.cut), (2, 2, 1, 1, 0));
        let h = highlights(&store);
        assert_eq!(patterns(&h.phrases), ["cyex", "hello", "hello", "a+"]);
        assert_eq!(h.me.alert.color, "#00ff00");
        assert!(h.me.alert.sound && !h.replies.alert.sound, "V106-09");
        assert_eq!(h.users[0].login, "bob");
        assert!(h.global && h.global_list[0].alert.sound_file == "C:\\s\\g.wav");
        assert!(!h.duplicates_asked);
        store.flush().unwrap();
        let saved: Value = serde_json::from_str(&file(tmp.path(), "highlights.json")).unwrap();
        assert_eq!(saved["mentionsMerged"], true);
        assert_eq!(file(tmp.path(), "mentions.json"), OLD_MENTIONS, "untouched");

        // Runs once.
        let (again, report) = open(tmp.path(), &clock);
        assert!(report.merged.is_none());
        assert_eq!(highlights(&again), h);
    }

    #[test]
    fn mentions_only_file_is_merged() {
        let tmp = TempDir::new("merge-mentions-only");
        write(tmp.path(), "mentions.json", OLD_MENTIONS);
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, report) = open(tmp.path(), &clock);
        assert_eq!(report.merged.unwrap().from_mentions, 2);
        let h = highlights(&store);
        assert_eq!(patterns(&h.phrases), ["cyex", "hello"]);
        assert_eq!(h.phrases[0].flavor, crate::schema::RegexFlavor::Python);
        assert!(h.global && h.me.alert.color == "#00ff00");
        store.flush().unwrap();
        let saved: Value = serde_json::from_str(&file(tmp.path(), "highlights.json")).unwrap();
        assert_eq!(saved["mentionsMerged"], true);
        assert_eq!(file(tmp.path(), "mentions.json"), OLD_MENTIONS);
    }

    #[test]
    fn unreadable_mentions_file_is_left_as_is() {
        let tmp = TempDir::new("merge-unreadable");
        write(tmp.path(), "highlights.json", OLD_HIGHLIGHTS);
        write(tmp.path(), "mentions.json", "{ nope");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (_, report) = open(tmp.path(), &clock);
        assert!(report.old_mentions_unreadable);
        assert_eq!(report.merged.unwrap().from_highlights, 2);
        assert_eq!(file(tmp.path(), "mentions.json"), "{ nope");
    }

    #[test]
    fn leftover_broken_mentions_not_flagged_after_merge() {
        // R20 Highlights already carry the merged mark: mentions.json isn't read.
        let tmp = TempDir::new("merge-leftover-broken");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.flush().unwrap();
        write(tmp.path(), "mentions.json", "{ nope");
        let (_, report) = open(tmp.path(), &clock);
        assert!(!report.old_mentions_unreadable);
        assert!(report.merged.is_none());
    }

    #[test]
    fn restore_with_unreadable_mentions_file_flags_it() {
        // R20 restore_from flags a broken snapshot mentions.json like open does.
        let tmp = TempDir::new("merge-restore-unreadable");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        let snap = backup::snapshot_dir(tmp.path(), day(9));
        fs::create_dir_all(&snap).unwrap();
        write(&snap, "highlights.json", OLD_HIGHLIGHTS);
        write(&snap, "mentions.json", "{ nope");
        store.restore_backup(day(9)).unwrap();
        assert!(store.old_mentions_unreadable());
        assert_eq!(patterns(&highlights(&store).phrases), ["hello", "a+"]);
    }

    #[test]
    fn reset_never_merges_again() {
        let tmp = TempDir::new("merge-reset");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.flush().unwrap();
        store.reset(Partition::Highlights);
        store.flush().unwrap();
        let (store, report) = open(tmp.path(), &clock);
        assert!(report.merged.is_none());
        assert_eq!(highlights(&store), Highlights::default());
    }

    #[test]
    fn backup_before_merge() {
        let tmp = TempDir::new("merge-backup");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.flush().unwrap();
        let snap = backup::snapshot_dir(tmp.path(), day(9));
        assert_eq!(file(&snap, "highlights.json"), OLD_HIGHLIGHTS);
        assert_eq!(file(&snap, "mentions.json"), OLD_MENTIONS);
    }

    #[test]
    fn name_and_replies_always_in_mentions_tab() {
        let tmp = TempDir::new("merge-show");
        write(tmp.path(), "mentions.json", r#"{"me": {"show": false}}"#);
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        assert!(highlights(&store).me.alert.show && highlights(&store).replies.alert.show);
        store
            .patch(Partition::Highlights, &json!({"me": {"show": false}, "replies": {"show": false}}))
            .unwrap();
        assert!(highlights(&store).me.alert.show && highlights(&store).replies.alert.show);
    }

    #[test]
    fn import_v105_bundle() {
        let tmp = TempDir::new("merge-import-v105");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        let parsed = |t: &str| serde_json::from_str::<Value>(t).unwrap();
        let got = store
            .import(bundle(json!({"highlights": parsed(OLD_HIGHLIGHTS), "mentions": parsed(OLD_MENTIONS)})))
            .unwrap();
        assert_eq!(got, vec![Partition::Highlights]);
        let h = highlights(&store);
        assert_eq!(patterns(&h.phrases), ["cyex", "hello", "hello", "a+"]);
        assert!(h.global && h.me.alert.sound && h.me.alert.color == "#00ff00");
    }

    #[test]
    fn import_mentions_only_bundle() {
        let tmp = TempDir::new("merge-import-mentions");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        let got = store
            .import(bundle(json!({"mentions": {"me": {"color": "#abcdef"},
                "list": [{"pattern": "cyex", "flavor": "python", "regex": true}, {"pattern": "new"}]}})))
            .unwrap();
        assert_eq!(got, vec![Partition::Highlights]);
        let h = highlights(&store);
        assert_eq!(patterns(&h.phrases), ["new", "cyex", "hello", "hello", "a+"], "exact copies skipped");
        assert_eq!(h.me.alert.color, "#abcdef");
        assert!(!h.global && h.global_list.is_empty(), "global replaced");
        assert_eq!(h.users[0].login, "bob", "the rest kept");
    }

    #[test]
    fn import_highlights_only_old_bundle() {
        let tmp = TempDir::new("merge-import-highlights");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.patch(Partition::Highlights, &json!({"me": {"sound": false}})).unwrap();
        let before = highlights(&store);
        store
            .import(bundle(
                json!({"highlights": {"phrases": [{"pattern": "x"}, {"pattern": "y+", "regex": true}]}}),
            ))
            .unwrap();
        let h = highlights(&store);
        assert_eq!(patterns(&h.phrases), ["x", "y+"]);
        assert!(h.phrases[0].whole_word && !h.phrases[1].whole_word);
        assert_eq!((&h.me, &h.replies), (&before.me, &before.replies));
        assert_eq!((h.global, &h.global_list), (before.global, &before.global_list));
    }

    #[test]
    fn share_code_old_bundle() {
        let tmp = TempDir::new("merge-share");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        let old = serde_json::from_str::<Value>(OLD_MENTIONS).unwrap();
        let code = bundle(json!({ "mentions": old })).to_share_code();
        store.import(Bundle::from_share_code(&code).unwrap()).unwrap();
        assert_eq!(patterns(&highlights(&store).phrases), ["cyex", "hello"]);
    }

    #[test]
    fn restore_v105_daily_backup() {
        let tmp = TempDir::new("merge-restore-v105");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        let merged = highlights(&store);
        store.reset(Partition::Highlights);
        store.restore_backup(day(9)).unwrap();
        assert_eq!(highlights(&store), merged);
    }

    #[test]
    fn restore_v106_snapshot_ignores_stale_mentions() {
        let tmp = TempDir::new("merge-restore-v106");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.flush().unwrap();
        let merged = highlights(&store);
        let name = store.snapshot_now().unwrap().unwrap();
        assert!(backup::named_dir(tmp.path(), &name).join("mentions.json").is_file());
        store.reset(Partition::Highlights);
        store.restore_snapshot(&name).unwrap();
        assert_eq!(highlights(&store), merged, "no second merge, no doubled words");
    }

    #[test]
    fn corrupt_highlights_still_folds_mentions() {
        // R7 The old mention words are kept on top of the fresh defaults, once.
        let tmp = TempDir::new("merge-corrupt-highlights");
        write(tmp.path(), "highlights.json", "{ nope");
        write(tmp.path(), "mentions.json", OLD_MENTIONS);
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, report) = open(tmp.path(), &clock);
        assert_eq!(report.reset, vec![Partition::Highlights]);
        assert!(report.merged.is_some());
        let kept = highlights(&store);
        assert!(patterns(&kept.phrases).contains(&"cyex"));
        store.flush().unwrap();
        let saved: Value = serde_json::from_str(&file(tmp.path(), "highlights.json")).unwrap();
        assert_eq!(saved["mentionsMerged"], true);
        let (again, report) = open(tmp.path(), &clock);
        assert!(report.merged.is_none());
        assert_eq!(highlights(&again), kept);
    }

    #[test]
    fn restore_with_misshaped_mentions() {
        let tmp = TempDir::new("merge-restore-bad-mentions");
        v105(tmp.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.flush().unwrap();
        let merged = highlights(&store);
        let snap = backup::snapshot_dir(tmp.path(), day(9));
        write(&snap, "highlights.json", OLD_HIGHLIGHTS);
        write(&snap, "mentions.json", "[1, 2]");
        store.reset(Partition::Highlights);
        assert!(!store.old_mentions_unreadable());
        store.restore_backup(day(9)).unwrap();
        assert!(store.old_mentions_unreadable());
        assert_eq!(patterns(&highlights(&store).phrases), ["hello", "a+"]);
        assert_ne!(highlights(&store), merged);
    }

    #[test]
    fn export_import_round_trip() {
        let a_dir = TempDir::new("merge-export-a");
        let b_dir = TempDir::new("merge-export-b");
        v105(a_dir.path());
        let clock = Arc::new(FakeClock::at(day(9)));
        let (a, _) = open(a_dir.path(), &clock);
        let out = a.export(&Partition::ALL);
        assert!(!out.partitions.contains_key("mentions"));
        let (mut b, _) = open(b_dir.path(), &clock);
        b.import(Bundle::from_json(&out.to_json()).unwrap()).unwrap();
        assert_eq!(highlights(&b), highlights(&a));
    }
}
