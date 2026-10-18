//! Checks against the real services, signed out. Run by hand:
//! `cargo test -p cyex-player --test live -- --ignored --nocapture`

use cyex_player::fetch::Twitch;
use cyex_player::live::{Live, Options};
use cyex_player::vod::Vod;
use cyex_twitch::{Http, gql};

const DEVICE: &str = "0123456789abcdefghijklmnopqrstuv";

async fn live_login(http: &Http) -> String {
    let ids: Vec<String> = ["71092938", "207813352", "121059319", "26261471"].map(String::from).to_vec();
    let list = gql::channels_by_ids(http, &ids).await.unwrap();
    list.into_iter().find(|c| c.stream.is_some()).expect("one of them live").login
}

#[tokio::test]
#[ignore = "network"]
async fn live_stream_through_the_filter() {
    let http = Http::new().unwrap();
    let login = live_login(&http).await;
    let fetch = Twitch {
        http: http.clone(),
        device_id: DEVICE.into(),
        proxy: None,
        sign_in: None,
        #[cfg(feature = "mocks")]
        mock: None,
    };
    let live = Live::new(fetch, &login, Options { low_latency: true, block_ads: true });
    let master = live.master().await.unwrap();
    println!("{login} master:\n{master}");
    let (first, ad) = live.media(0).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let (second, _) = live.media(0).await.unwrap();
    println!("ad: {ad:?}\n{second}");
    assert!(first.contains("#EXTINF") && second.contains("#EXTINF"));
}

#[tokio::test]
#[ignore = "network"]
async fn vod_through_the_proxy() {
    let http = Http::new().unwrap();
    let login = live_login(&http).await;
    let videos = gql::channel_videos(&http, &login, Some(gql::VideoKind::Archive), gql::VideoSort::Newest)
        .await
        .unwrap();
    let id = &videos.iter().find(|v| !v.recording).expect("a finished VOD").id;
    let vod = Vod::new(
        Twitch {
            http,
            device_id: DEVICE.into(),
            proxy: None,
            sign_in: None,
            #[cfg(feature = "mocks")]
            mock: None,
        },
        id,
    );
    let master = vod.master().await.unwrap();
    let media = vod.media(0).await.unwrap();
    let first = media.lines().find(|l| !l.starts_with('#') && !l.is_empty()).unwrap();
    let bytes = vod.segment(0, first).await.unwrap();
    println!("{id}: {} variants, first segment {first} {} bytes", master.matches("STREAM-INF").count(), bytes.len());
    assert_eq!(bytes[0], 0x47, "MPEG-TS");
}
