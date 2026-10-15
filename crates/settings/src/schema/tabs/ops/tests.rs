use super::super::testing::{counter, order, tab};
use super::*;

fn tabs(ids: &[&str]) -> Tabs {
    Tabs {
        tabs: ids.iter().map(|id| tab(id, id)).collect(),
        active: ids.first().map(|s| (*s).to_owned()),
        ..Tabs::default()
    }
}

fn apply(t: &mut Tabs, op: TabOp) -> Option<String> {
    t.apply(op, &mut counter()).unwrap()
}

fn channel(login: &str) -> TabChannel {
    TabChannel {
        id: format!("u{login}"),
        login: login.into(),
        name: login.into(),
    }
}

fn add(t: &mut Tabs, login: &str, into: Option<&str>) -> String {
    let op = TabOp::Add {
        channel: channel(login),
        show: TileShow {
            player: false,
            chat: true,
        },
        account: Some("me".into()),
        alias: None,
        into: into.map(Into::into),
    };
    apply(t, op).unwrap()
}

/// Channel logins per column, left to right.
fn layout(t: &Tab) -> Vec<Vec<&str>> {
    t.columns
        .iter()
        .map(|c| c.tiles.iter().map(|x| x.channel.login.as_str()).collect())
        .collect()
}

#[test]
fn adding_opens_a_new_tab_and_remembers_the_choice() {
    let mut t = tabs(&["a"]);
    let id = add(&mut t, "xqc", None);
    assert_eq!(order(&t), ["a", id.as_str()]);
    assert_eq!(t.active.as_deref(), Some(id.as_str()));
    assert!(!t.last_show.player);
    let tile = &t.tabs[1].columns[0].tiles[0];
    assert_ne!(tile.id, id);
    assert_eq!(tile.account.as_deref(), Some("me"));
}

#[test]
fn adding_many_makes_a_grid() {
    let mut t = tabs(&["a"]);
    let many = |logins: &[&str]| TabOp::AddMany {
        channels: logins.iter().map(|l| channel(l)).collect(),
        show: TileShow {
            player: true,
            chat: false,
        },
        account: None,
    };
    let id = apply(&mut t, many(&["x", "y", "z"])).unwrap();
    assert_eq!(t.active.as_deref(), Some(id.as_str()));
    assert_eq!(layout(t.tab(&id).unwrap()), [["x"], ["y"], ["z"]]);
    let muted: Vec<bool> = t.tab(&id).unwrap().tiles().map(|x| x.muted).collect();
    assert_eq!(muted, [false, true, true]);
    assert!(t.last_show.chat);

    let id = apply(&mut t, many(&["1", "2", "3", "4", "5"])).unwrap();
    assert_eq!(
        layout(t.tab(&id).unwrap()),
        [vec!["1", "4"], vec!["2", "5"], vec!["3"]]
    );
    let id = apply(&mut t, many(&["1", "2", "3", "4"])).unwrap();
    assert_eq!(layout(t.tab(&id).unwrap()), [["1", "3"], ["2", "4"]]);
    let lots: Vec<String> = (0..12).map(|i| i.to_string()).collect();
    let lots: Vec<&str> = lots.iter().map(String::as_str).collect();
    let id = apply(&mut t, many(&lots)).unwrap();
    assert_eq!(t.tab(&id).unwrap().tiles().count(), MAX_MULTI);
    assert_eq!(t.apply(many(&[]), &mut counter()), Err(NO_CHANNELS));
}

#[test]
fn adding_into_a_tab_tiles_it() {
    let mut t = tabs(&["a"]);
    assert_eq!(add(&mut t, "lirik", Some("a")), "a");
    assert_eq!(layout(&t.tabs[0]), [["a"], ["lirik"]]);
    let gone = TabOp::Add {
        channel: channel("x"),
        show: TileShow::default(),
        account: None,
        alias: None,
        into: Some("nope".into()),
    };
    assert_eq!(t.apply(gone, &mut counter()), Err(GONE_TAB));
}

#[test]
fn editing_changes_one_tile() {
    let mut t = tabs(&["a"]);
    let op = TabOp::Edit {
        tile: "a-1".into(),
        channel: channel("pokimane"),
        show: TileShow {
            player: true,
            chat: false,
        },
        account: None,
    };
    apply(&mut t, op);
    let tile = &t.tabs[0].columns[0].tiles[0];
    assert_eq!(tile.channel.login, "pokimane");
    assert!(tile.show.player && !tile.show.chat);
    assert_eq!(tile.account, None);
}

#[test]
fn showing_hides_chat_or_stream_without_changing_the_default() {
    let mut t = tabs(&["a"]);
    let before = t.last_show;
    let show = |player, chat| TabOp::Show {
        tile: "a-1".into(),
        show: TileShow { player, chat },
    };
    apply(&mut t, show(true, false));
    let tile = &t.tabs[0].columns[0].tiles[0];
    assert!(tile.show.player && !tile.show.chat);
    assert_eq!(t.last_show, before);
    // Never neither.
    apply(&mut t, show(false, false));
    assert!(t.tabs[0].columns[0].tiles[0].show.player);
}

#[test]
fn closing_shows_a_neighbor() {
    let mut t = tabs(&["a", "b", "c", "d"]);
    apply(&mut t, TabOp::Activate { tab: Some("b".into()) });
    apply(
        &mut t,
        TabOp::Close {
            tabs: vec!["b".into(), "c".into()],
        },
    );
    assert_eq!(order(&t), ["a", "d"]);
    assert_eq!(t.active.as_deref(), Some("d"));
    apply(&mut t, TabOp::CloseTile { tile: "d-1".into() });
    assert_eq!(order(&t), ["a"]);
    assert_eq!(t.active.as_deref(), Some("a"));
}

#[test]
fn renaming_and_clearing_an_alias() {
    let mut t = tabs(&["a"]);
    let rename = |alias: Option<&str>| TabOp::Rename {
        tab: "a".into(),
        alias: alias.map(Into::into),
    };
    apply(&mut t, rename(Some("  🎮 my tab ")));
    assert_eq!(t.tabs[0].label(), "🎮 my tab");
    apply(&mut t, rename(Some("")));
    assert_eq!(t.tabs[0].label(), "A");
}

#[test]
fn moving_a_tab() {
    let mut t = tabs(&["a", "b", "c"]);
    let mv = |tab: &str, before: Option<&str>| TabOp::Move {
        tab: tab.into(),
        before: before.map(Into::into),
        group: None,
    };
    apply(&mut t, mv("c", Some("a")));
    assert_eq!(order(&t), ["c", "a", "b"]);
    apply(&mut t, mv("c", None));
    assert_eq!(order(&t), ["a", "b", "c"]);
}

#[test]
fn groups_keep_their_tabs_together() {
    let mut t = tabs(&["a", "b", "c", "d"]);
    let g = apply(
        &mut t,
        TabOp::NewGroup {
            tabs: vec!["a".into(), "c".into()],
            name: "games".into(),
            color: GroupColor::Blue,
        },
    )
    .unwrap();
    assert_eq!(order(&t), ["a", "c", "b", "d"]);
    apply(
        &mut t,
        TabOp::Group {
            tabs: vec!["d".into()],
            group: Some(g.clone()),
        },
    );
    assert_eq!(order(&t), ["a", "c", "d", "b"]);

    // Dragged out of the group and dropped at the end.
    apply(
        &mut t,
        TabOp::Move {
            tab: "a".into(),
            before: None,
            group: None,
        },
    );
    assert_eq!(order(&t), ["c", "d", "b", "a"]);

    apply(
        &mut t,
        TabOp::EditGroup {
            group: g.clone(),
            name: Some("fps".into()),
            color: None,
            collapsed: Some(true),
        },
    );
    assert_eq!((t.groups[0].name.as_str(), t.groups[0].collapsed), ("fps", true));
    assert_eq!(t.groups[0].color, GroupColor::Blue);

    apply(&mut t, TabOp::Ungroup { group: g });
    assert!(t.groups.is_empty());
    assert!(t.tabs.iter().all(|x| x.group.is_none()));
    let none = TabOp::NewGroup {
        tabs: vec!["zz".into()],
        name: String::new(),
        color: GroupColor::Red,
    };
    assert_eq!(t.apply(none, &mut counter()), Err(NO_TABS));
}

#[test]
fn empty_groups_disappear() {
    let mut t = tabs(&["a", "b"]);
    apply(
        &mut t,
        TabOp::NewGroup {
            tabs: vec!["a".into()],
            name: "x".into(),
            color: GroupColor::Red,
        },
    );
    apply(&mut t, TabOp::Close { tabs: vec!["a".into()] });
    assert!(t.groups.is_empty());
}

#[test]
fn tiles_move_beside_or_swap_with_others() {
    let mut t = tabs(&["a", "b"]);
    add(&mut t, "c", Some("a"));
    let tile_of = |t: &Tabs, login: &str| t.tiles().find(|x| x.channel.login == login).unwrap().id.clone();
    let mv = |t: &mut Tabs, from: &str, to: &str, zone: Zone| {
        let op = TabOp::MoveTile {
            tile: tile_of(t, from),
            target: tile_of(t, to),
            zone,
        };
        apply(t, op);
    };

    // Tab b's only tile moves below a: tab b closes.
    mv(&mut t, "b", "a", Zone::Bottom);
    assert_eq!(order(&t), ["a"]);
    assert_eq!(layout(&t.tabs[0]), [vec!["a", "b"], vec!["c"]]);
    let sizes: Vec<f32> = t.tabs[0].columns[0].tiles.iter().map(|x| x.size).collect();
    assert_eq!(sizes, [1.0, 1.0]);

    mv(&mut t, "c", "a", Zone::Top);
    assert_eq!(layout(&t.tabs[0]), [["c", "a", "b"]]);
    mv(&mut t, "b", "c", Zone::Left);
    assert_eq!(layout(&t.tabs[0]), [vec!["b"], vec!["c", "a"]]);
    mv(&mut t, "b", "a", Zone::Right);
    assert_eq!(layout(&t.tabs[0]), [vec!["c", "a"], vec!["b"]]);
    mv(&mut t, "b", "c", Zone::Center);
    assert_eq!(layout(&t.tabs[0]), [vec!["b", "a"], vec!["c"]]);
    assert_eq!(t.active.as_deref(), Some("a"));
}

#[test]
fn tiles_detach_into_tabs_and_back() {
    let mut t = tabs(&["a", "b"]);
    add(&mut t, "c", Some("a"));
    let c = t.tabs[0].columns[1].tiles[0].id.clone();

    let id = apply(
        &mut t,
        TabOp::Detach {
            tile: c.clone(),
            before: Some("b".into()),
        },
    )
    .unwrap();
    assert_eq!(order(&t), ["a", id.as_str(), "b"]);
    assert_eq!(t.active.as_deref(), Some(id.as_str()));

    apply(&mut t, TabOp::SendTile { tile: c, tab: "b".into() });
    assert_eq!(order(&t), ["a", "b"]);
    assert_eq!(layout(&t.tabs[1]), [["b"], ["c"]]);
    // What stays behind fills its tab again.
    assert_eq!(t.tabs[0].columns[0].size, 1.0);
    assert_eq!(t.active.as_deref(), Some("b"));

    // A tab's only tile "detached" just moves the tab.
    apply(
        &mut t,
        TabOp::Detach {
            tile: "a-1".into(),
            before: None,
        },
    );
    assert_eq!(order(&t), ["b", "a"]);
}

#[test]
fn resizing_needs_the_same_layout() {
    let mut t = tabs(&["a"]);
    add(&mut t, "c", Some("a"));
    let resize = |columns: Vec<f32>, rows: Vec<Vec<f32>>| TabOp::Resize {
        tab: "a".into(),
        columns,
        rows,
    };
    let share = |t: &Tabs| t.tabs[0].columns[0].size / t.tabs[0].columns[1].size;
    apply(&mut t, resize(vec![2.0, 1.0], vec![vec![1.0], vec![1.0]]));
    assert!((share(&t) - 2.0).abs() < 1e-5);
    apply(&mut t, resize(vec![9.0], vec![vec![1.0]]));
    assert!((share(&t) - 2.0).abs() < 1e-5);

    apply(
        &mut t,
        TabOp::Ratio {
            tile: "a-1".into(),
            ratio: 0.4,
        },
    );
    assert_eq!((t.tabs[0].columns[0].tiles[0].ratio, t.last_ratio), (0.4, 0.4));

    apply(&mut t, TabOp::Sound { tile: "a-1".into(), volume: 0.8, muted: true, boost: false });
    let tile = &t.tabs[0].columns[0].tiles[0];
    assert_eq!((tile.volume, tile.muted, t.last_volume), (0.8, true, 0.8));
    apply(&mut t, TabOp::Sound { tile: "a-1".into(), volume: 7.0, muted: false, boost: false });
    assert_eq!(t.tabs[0].columns[0].tiles[0].volume, 1.0, "clamped");

    // V105-10 The boost lifts the cap to 500 % and keeps new tiles at 100 % at most.
    apply(&mut t, TabOp::Sound { tile: "a-1".into(), volume: 3.0, muted: false, boost: true });
    let tile = &t.tabs[0].columns[0].tiles[0];
    assert_eq!((tile.volume, tile.boost, t.last_volume), (3.0, true, 1.0));
    apply(&mut t, TabOp::Sound { tile: "a-1".into(), volume: 9.0, muted: false, boost: true });
    assert_eq!(t.tabs[0].columns[0].tiles[0].volume, 5.0);
    apply(&mut t, TabOp::Sound { tile: "a-1".into(), volume: 3.0, muted: false, boost: false });
    assert_eq!(t.tabs[0].columns[0].tiles[0].volume, 1.0);
}

#[test]
fn feeds_and_filters() {
    let mut t = Tabs::default();
    apply(&mut t, TabOp::AddFeed { name: " Pings ".into() });
    let tile = &t.tabs[0].columns[0].tiles[0];
    assert_eq!((tile.feed.as_deref(), tile.show.player), (Some("Pings"), false));
    assert!(t.channel_ids().is_empty(), "feeds have no channel");
    let id = tile.id.clone();
    apply(&mut t, TabOp::Filters { tile: id, filters: vec!["f1".into(), "f1".into()] });
    assert_eq!(t.tabs[0].columns[0].tiles[0].filters, ["f1"]);
    let mut fresh = || "x".to_owned();
    assert!(t.apply(TabOp::AddFeed { name: " ".into() }, &mut fresh).is_err());
}

#[test]
fn ops_parse_from_the_ui() {
    let op: TabOp = serde_json::from_str(r#"{"op":"moveTile","tile":"x","target":"y","zone":"left"}"#).unwrap();
    assert!(matches!(op, TabOp::MoveTile { zone: Zone::Left, .. }));
    let op: TabOp = serde_json::from_str(r#"{"op":"activate","tab":null}"#).unwrap();
    assert!(matches!(op, TabOp::Activate { tab: None }));
    let op: TabOp = serde_json::from_str(r#"{"op":"chatSide","tile":"x","side":"bottom"}"#).unwrap();
    assert!(matches!(op, TabOp::ChatSide { side: Some(ChatPosition::Bottom), .. }));
}

#[test]
fn dragged_chat_sides_stay_per_tile_until_reset() {
    let mut t = tabs(&["a", "b"]);
    add(&mut t, "c", Some("a"));
    let side = |t: &Tabs, tab: usize, col: usize| t.tabs[tab].columns[col].tiles[0].chat_side;
    let (a1, c) = (t.tabs[0].columns[0].tiles[0].id.clone(), t.tabs[0].columns[1].tiles[0].id.clone());
    let b1 = t.tabs[1].columns[0].tiles[0].id.clone();
    for (tile, s) in [(&a1, ChatPosition::Bottom), (&b1, ChatPosition::Left)] {
        apply(&mut t, TabOp::ChatSide { tile: tile.clone(), side: Some(s) });
    }
    // V102-23 Moving one chat leaves the others where they were.
    assert_eq!((side(&t, 0, 0), side(&t, 0, 1)), (Some(ChatPosition::Bottom), None));

    // It goes with its tile when tiles swap, and is kept on disk.
    apply(&mut t, TabOp::MoveTile { tile: c, target: a1, zone: Zone::Center });
    assert_eq!((side(&t, 0, 0), side(&t, 0, 1)), (None, Some(ChatPosition::Bottom)));
    let back: Tabs = serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap();
    assert_eq!(side(&back, 0, 1), Some(ChatPosition::Bottom));

    // V102-26 The mod view's divider is kept with the tile too, and reset with it.
    apply(&mut t, TabOp::ModviewShare { tile: b1.clone(), share: Some(0.6) });
    let share = |t: &Tabs, tab: usize| t.tabs[tab].columns[0].tiles[0].modview_share;
    assert_eq!(share(&t, 1), Some(0.6));
    apply(&mut t, TabOp::ModviewShare { tile: b1.clone(), share: Some(7.0) });
    assert_eq!(share(&t, 1), Some(0.9), "clamped");
    let op: TabOp = serde_json::from_str(r#"{"op":"modviewShare","tile":"x","share":null}"#).unwrap();
    assert!(matches!(op, TabOp::ModviewShare { share: None, .. }));

    apply(&mut t, TabOp::ResetLayout { tab: "a".into() });
    assert_eq!((side(&t, 0, 0), side(&t, 0, 1)), (None, None));
    assert_eq!(side(&t, 1, 0), Some(ChatPosition::Left), "only that tab");
    apply(&mut t, TabOp::ResetLayout { tab: "b".into() });
    assert_eq!(share(&t, 1), None);
    let mut fresh = || "x".to_owned();
    assert!(t.apply(TabOp::ResetLayout { tab: "gone".into() }, &mut fresh).is_err());
}

#[test]
fn hand_made_panel_layouts_are_kept_per_tile_until_reset() {
    let mut t = tabs(&["a"]);
    let tile = t.tabs[0].columns[0].tiles[0].id.clone();
    let op: TabOp = serde_json::from_value(serde_json::json!({
        "op": "layout", "tile": tile,
        "layout": {"dir": "row", "parts": [{"panel": "modview"}, {"dir": "column", "parts": [{"panel": "stream"}, {"panel": "chat"}], "sizes": [3, 1]}], "sizes": [1, 1]}
    }))
    .unwrap();
    apply(&mut t, op);
    let layout = |t: &Tabs| t.tabs[0].columns[0].tiles[0].layout.clone();
    // V103-05 Stored as given, sizes made shares.
    let Some(crate::schema::PanelNode::Split { parts, sizes, .. }) = layout(&t) else { panic!() };
    assert_eq!((parts.len(), sizes), (2, vec![0.5, 0.5]));
    let back: Tabs = serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap();
    assert_eq!(layout(&back), layout(&t));
    apply(&mut t, TabOp::ResetLayout { tab: "a".into() });
    assert_eq!(layout(&t), None);
}
