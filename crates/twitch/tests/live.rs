//! Checks against the real services, signed out. Run by hand:
//! `cargo test -p cyex-twitch --test live -- --ignored`

use cyex_twitch::{Http, gql, recent};

#[tokio::test]
#[ignore = "network"]
async fn chat_catalogs() {
    let http = Http::new().unwrap();
    let global = gql::global_chat(&http).await.unwrap();
    assert!(global.badges.len() > 100);
    assert!(global.cheermotes.iter().any(|c| c.prefix == "Cheer"));
    let ch = gql::channel_chat(&http, "xqc", &global.cheer_colors).await.unwrap().unwrap();
    assert_eq!(ch.channel_id, "71092938");
    assert!(ch.badges.iter().any(|b| b.set == "subscriber"));
    let points = ch.points.unwrap();
    println!("points: {:?} {} custom, {} automatic", points.name, points.custom.len(), points.automatic.len());
    let pin = gql::pinned(&http, &ch.channel_id).await.unwrap();
    println!("pinned: {:?}", pin.map(|p| p.kind));
    let p = gql::profile(&http, Some("twitch"), None).await.unwrap().unwrap();
    assert!(p.partner && p.followers.unwrap() > 0);
    let lines = recent::recent_messages(&http, "twitch").await.unwrap();
    assert!(lines.iter().all(|l| l.contains("historical=1")));
}

#[tokio::test]
#[ignore = "network"]
async fn channel_pages() {
    use gql::{ClipPeriod, ClipSort, VideoSort};
    let http = Http::new().unwrap();
    let about = gql::about(&http, "zackrawrr").await.unwrap().unwrap();
    println!(
        "about: {} followers, {} socials, {} panels, last {:?}",
        about.followers,
        about.socials.len(),
        about.panels.len(),
        about.last_stream.as_ref().map(|l| &l.started_at)
    );
    assert!(about.followers > 0);
    let clips = gql::channel_clips(&http, "zackrawrr", ClipPeriod::Week, ClipSort::Views).await.unwrap();
    println!("clips: {}", clips.len());
    let slug = &clips.first().expect("a clip this week").slug;
    let play = gql::clip(&http, slug).await.unwrap().unwrap();
    println!("clip: {} {:?} vod {:?}", play.clip.title, play.qualities.iter().map(|q| q.height).collect::<Vec<_>>(), play.vod_id);
    assert!(play.qualities[0].url.contains("sig="));
    let videos = gql::channel_videos(&http, "zackrawrr", None, VideoSort::Newest).await.unwrap();
    println!("videos: {} first {:?}", videos.len(), videos.first().map(|v| (&v.title, v.kind, v.recording)));
    let one = gql::video(&http, &videos[0].id).await.unwrap().unwrap();
    assert_eq!(one.channel_login, "zackrawrr");
}

#[tokio::test]
#[ignore = "network"]
async fn playback() {
    use cyex_twitch::usher;
    use gql::{Auth, Player};
    let http = Http::new().unwrap();
    let ids: Vec<String> = ["71092938", "207813352", "121059319", "26261471"].map(String::from).to_vec();
    let live = gql::channels_by_ids(&http, &ids).await.unwrap();
    let login = &live.iter().find(|c| c.stream.is_some()).expect("one of them live").login;
    let token = gql::stream_token(&http, Auth::PUBLIC, None, login, Player::SITE).await.unwrap();
    let master = usher::text(&http, &usher::live_url(login, &token, "h264")).await.unwrap();
    assert!(master.starts_with("#EXTM3U") && master.contains("#EXT-X-STREAM-INF"));
    let media_url = master.lines().find(|l| l.starts_with("https://")).unwrap();
    let media = usher::text(&http, media_url).await.unwrap();
    println!("{login}: {} lines, prefetch {}", media.lines().count(), media.contains("PREFETCH"));
    assert!(media.contains("#EXTINF"));

    let offline = gql::stream_token(&http, Auth::PUBLIC, None, "twitch", Player::SITE).await.unwrap();
    let err = usher::text(&http, &usher::live_url("twitch", &offline, "h264")).await;
    println!("offline: {:?}", err.as_ref().map_err(|e| e.api_message()));
}

#[tokio::test]
#[ignore = "network"]
async fn tab_channels() {
    let http = Http::new().unwrap();
    let ids: Vec<String> = ["71092938", "44445592", "1"].map(String::from).to_vec();
    let list = gql::channels_by_ids(&http, &ids).await.unwrap();
    assert!(list.iter().any(|c| c.login == "xqc"));
    println!("live: {:?}", list.iter().map(|c| (&c.login, c.stream.is_some())).collect::<Vec<_>>());
    let found = gql::lookup(&http, "https://www.twitch.tv/pokimane").await.unwrap();
    assert_eq!(found[0].id, "44445592");
    let both = gql::lookup(&http, "71092938").await.unwrap();
    assert_eq!(both.len(), 2);
    let none = gql::lookup(&http, "id:1").await.unwrap();
    println!("id 1: {:?}", none.first().map(|c| &c.login));
}

#[tokio::test]
#[ignore = "network"]
async fn discover() {
    use gql::{Auth, StreamFilter, StreamSort};
    let http = Http::new().unwrap();
    let all = StreamFilter::default();
    let top = gql::top_streams(&http, Auth::PUBLIC, &all, None).await.unwrap();
    assert!(top.items.len() > 20 && top.cursor.is_some());
    let first = &top.items[0];
    println!(
        "top: {} streams, first {} {:?}",
        top.items.len(),
        first.login,
        first.stream.as_ref().map(|s| s.viewers)
    );
    // Signed out, Twitch serves only the first page.
    let next = gql::top_streams(&http, Auth::PUBLIC, &all, top.cursor.as_deref()).await;
    println!(
        "page 2 signed out: {:?}",
        next.as_ref()
            .map(|p| p.items.len())
            .map_err(|e| e.api_message().map(str::to_owned))
    );

    let german = StreamFilter {
        sort: StreamSort::Recent,
        languages: vec!["DE".into()],
        tag: None,
    };
    let de = gql::top_streams(&http, Auth::PUBLIC, &german, None)
        .await
        .unwrap();
    println!("newest german: {}", de.items.len());
    let tagged = StreamFilter {
        tag: Some("vtuber".into()),
        ..StreamFilter::default()
    };
    let vt = gql::top_streams(&http, Auth::PUBLIC, &tagged, None)
        .await
        .unwrap();
    assert!(vt.items.iter().all(|c| {
        c.stream
            .as_ref()
            .unwrap()
            .tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case("vtuber"))
    }));

    let cats = gql::top_categories(&http, Auth::PUBLIC, None).await.unwrap();
    assert!(cats.items.len() > 50);
    let slug = &cats.items[0].slug;
    let cat = gql::category(&http, Auth::PUBLIC, slug, &all, None)
        .await
        .unwrap()
        .unwrap();
    println!(
        "{}: {} viewers, {} streams",
        cat.category.name,
        cat.category.viewers,
        cat.streams.items.len()
    );
    assert!(
        cat.streams
            .items
            .iter()
            .all(|c| c.stream.as_ref().unwrap().game.as_ref().unwrap().slug == *slug)
    );
    assert!(
        gql::category(&http, Auth::PUBLIC, "no-such-category-x9", &all, None)
            .await
            .unwrap()
            .is_none()
    );

    let found = gql::search(&http, Auth::PUBLIC, "chess").await.unwrap();
    println!(
        "search chess: {} channels, {} categories",
        found.channels.items.len(),
        found.categories.len()
    );
    let slugs: Vec<&str> = found.categories.iter().take(3).map(|c| c.slug.as_str()).collect();
    println!("categories: {slugs:?}");
    assert!(found.categories.iter().any(|c| c.name == "Chess"));
    let more = gql::search_channels(
        &http,
        Auth::PUBLIC,
        "chess",
        found.channels.cursor.as_deref().unwrap(),
    )
    .await
    .unwrap();
    assert!(!more.items.is_empty());
    assert!(
        more.items
            .iter()
            .all(|c| !found.channels.items.iter().any(|f| f.id == c.id))
    );

    let ids = vec![
        "71092938".to_owned(),
        "999999999999".to_owned(),
        "12826".to_owned(),
    ];
    let cards = gql::cards(&http, &ids).await.unwrap();
    assert_eq!(cards.len(), 2);
    println!("xqc last live {:?}", cards[0].last_live);
}
