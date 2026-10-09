//! Read-only checks with the default account's GQL sign-in, from this PC's credential
//! store. Never prints the token and never writes anything to Twitch. Run by hand:
//! `cargo test -p cyex-twitch --test signed_in -- --ignored --nocapture`

use cyex_twitch::gql::{self, Auth, Player};
use cyex_twitch::{Http, usher};

/// The default account's GQL token and its client ID.
fn gql_sign_in() -> (String, String) {
    let dir = std::env::var("APPDATA").unwrap();
    let accounts: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{dir}/cyex-viewer/settings/accounts.json")).unwrap())
            .unwrap();
    let id = accounts["defaultId"].as_str().unwrap();
    let secret = keyring::Entry::new("cyex-viewer", &format!("{id}.gql"))
        .and_then(|e| e.get_password())
        .unwrap();
    let secret: serde_json::Value = serde_json::from_str(&secret).unwrap();
    (secret["access"].as_str().unwrap().to_owned(), secret["client_id"].as_str().unwrap().to_owned())
}

#[tokio::test]
#[ignore = "network, signed in"]
async fn stories_and_signed_in_playback() {
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };

    let logins = [
        "pokimane", "kaicenat", "jynxzi", "caseoh_", "valkyrae", "ludwig", "hasanabi", "xqc", "emiru", "fanum",
        "tarik", "shroud", "sketch", "ibai", "zackrawrr", "stableronaldo",
    ];
    let mut ids = Vec::new();
    for login in logins {
        if let Some(c) = gql::lookup(&http, login).await.unwrap().into_iter().next() {
            ids.push((c.login, c.id, c.stream.is_some()));
        }
    }
    let mut found = None;
    for (login, id, _) in &ids {
        match gql::story_ring(&http, auth, id).await {
            Ok(Some(ring)) => {
                println!("{login}: stories, unseen {}", ring.unseen);
                found.get_or_insert((login.clone(), id.clone()));
            }
            Ok(None) => println!("{login}: none"),
            Err(e) => println!("{login}: error {:?}", e.api_message()),
        }
    }
    if let Some((login, id)) = found {
        let list = gql::stories(&http, auth, &id).await.unwrap();
        for s in &list {
            let host = |u: &Option<String>| u.as_deref().and_then(|u| u.split('/').nth(2)).map(str::to_owned);
            println!(
                "{login} story {}: video {:?} image {:?} subs {} caption {:?} clip {:?}",
                s.created_at,
                host(&s.video),
                host(&s.image),
                s.subs_only,
                s.caption,
                s.clip
            );
        }
    }

    // Signed-in playback token for a live channel, with the token's own client.
    if let Some((login, _, _)) = ids.iter().find(|(_, _, live)| *live) {
        match gql::stream_token(&http, auth, None, login, Player { player_type: "popout", platform: "web" }).await {
            Ok(t) => {
                let v: serde_json::Value = serde_json::from_str(&t.value).unwrap();
                println!(
                    "{login} signed-in token: user {} hide_ads {} max {}",
                    v["user_id"].is_number() || v["user_id"].is_string(),
                    v["hide_ads"],
                    v["maximum_resolution"]
                );
                let master = usher::text(&http, &usher::live_url(login, &t, "h264")).await;
                println!("usher: {:?}", master.map(|m| m.matches("STREAM-INF").count()).map_err(|e| e.api_message().map(str::to_owned)));
            }
            Err(e) => println!("{login} signed-in token refused: {:?}", e.api_message()),
        }
    }
}

/// Reads the account's follows, live follows and followed categories. Prints counts only.
#[tokio::test]
#[ignore = "network, signed in"]
async fn follows() {
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let ids = gql::followed_ids(&http, auth).await.unwrap().unwrap();
    let cards = gql::cards(&http, &ids).await.unwrap();
    let live = gql::followed_live(&http, auth).await.unwrap().unwrap();
    let games = gql::followed_categories(&http, auth).await.unwrap();
    println!(
        "follows {} (cards {}, live in cards {}), live {}, never live {}, categories {}",
        ids.len(),
        cards.len(),
        cards.iter().filter(|c| c.stream.is_some()).count(),
        live.len(),
        cards.iter().filter(|c| c.last_live.is_none()).count(),
        games.len()
    );
    // Whether a signed-in token gets past the paging check signed-out requests fail.
    let top = gql::top_streams(&http, auth, &gql::StreamFilter::default(), None)
        .await
        .unwrap();
    let next = gql::top_streams(&http, auth, &gql::StreamFilter::default(), top.cursor.as_deref()).await;
    println!(
        "top streams page 2 signed in: {:?}",
        next.map(|p| p.items.len())
            .map_err(|e| e.api_message().map(str::to_owned))
    );
}
