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

/// 1.26, 1.28, 2.2 reads: whisper threads, the bell, Twitch's notification settings, the
/// profile, live alerts and subs. Prints counts and flags only, never message text.
#[tokio::test]
#[ignore = "network, signed in"]
async fn phase12_reads() {
    use gql::{notifications, profile, subs, whispers};
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let me = profile::profile(&http, auth).await.unwrap();
    println!(
        "profile: bio {} chars, color set {}, any color {}, language {:?}",
        me.bio.chars().count(),
        !me.color.is_empty(),
        me.any_color,
        me.language
    );
    match whispers::threads(&http, auth, &me.id, None).await {
        Ok(t) => {
            println!(
                "whispers: {} threads, more {}, unread {}, spam {}, block strangers {}",
                t.threads.len(),
                t.cursor.is_some(),
                t.threads.iter().map(|x| x.unread).sum::<u32>(),
                t.threads.iter().filter(|x| x.spam).count(),
                t.block_strangers
            );
            if let Some(first) = t.threads.first() {
                let page = whispers::thread(&http, auth, &me.id, &first.id, whispers::MESSAGES_PAGE, None).await.unwrap();
                let (_, msgs) = page.unwrap();
                println!(
                    "thread: {} messages, more {}, with nonce {}, emotes {}",
                    msgs.messages.len(),
                    msgs.cursor.is_some(),
                    msgs.messages.iter().filter(|m| m.nonce.is_some()).count(),
                    msgs.messages.iter().map(|m| m.emotes.len()).sum::<usize>()
                );
            }
        }
        Err(e) => println!("whispers: error {:?}", e.api_message()),
    }
    match notifications::list(&http, auth, notifications::List::Viewer, None).await {
        Ok(p) => println!(
            "bell: {} items, more {}, unread {}, with url {}, kinds {:?}",
            p.items.len(),
            p.cursor.is_some(),
            p.items.iter().filter(|i| !i.read).count(),
            p.items.iter().filter(|i| i.url.is_some()).count(),
            p.items.iter().map(|i| i.kind.as_str()).collect::<std::collections::BTreeSet<_>>()
        ),
        Err(e) => println!("bell: error {:?}", e.api_message()),
    }
    println!("bell unseen: {:?}", notifications::unseen(&http, auth).await.map_err(|e| e.api_message().map(str::to_owned)));
    match notifications::settings(&http, auth, None).await {
        Ok(s) => println!(
            "twitch notify: {} switches, categories {:?}, {} channels, more {}, states {:?}",
            s.switches.len(),
            s.switches.iter().map(|x| x.category.as_str()).collect::<std::collections::BTreeSet<_>>(),
            s.channels.len(),
            s.cursor.is_some(),
            s.channels.iter().map(|c| c.state.as_str()).collect::<std::collections::BTreeSet<_>>()
        ),
        Err(e) => println!("twitch notify: error {:?}", e.api_message()),
    }
    // V101-06 The master switches and the platforms they come in.
    if let Ok(s) = notifications::settings(&http, auth, None).await {
        let masters = s.switches.iter().filter(|x| x.category == "ALL" || x.category.ends_with("_category"));
        println!("twitch notify masters: {:?}", masters.map(|x| format!("{}/{}={}", x.category, x.platform, x.on)).collect::<Vec<_>>());
        println!(
            "twitch notify platforms: {:?}",
            s.switches.iter().map(|x| x.platform.as_str()).collect::<std::collections::BTreeSet<_>>()
        );
    }
    match gql::followed_live_alerts(&http, auth).await {
        Ok(Some(l)) => println!("live alerts: {} live, bell on {}", l.len(), l.iter().filter(|a| a.bell).count()),
        other => println!("live alerts: {:?}", other.map_err(|e| e.api_message().map(str::to_owned)).map(|o| o.map(|l| l.len()))),
    }
    match subs::mine(&http, auth).await {
        Ok(s) => println!("subs: {} active, {} expired, prime {}", s.active.len(), s.expired.len(), s.has_prime),
        Err(e) => println!("subs: error {:?}", e.api_message()),
    }
    match subs::channel(&http, auth, "xqc").await {
        Ok(Some(c)) => println!(
            "sub in xqc: subbed {}, resub {}, can prime {}, sells {}",
            c.sub.is_some(),
            c.resub.is_some(),
            c.can_prime,
            c.sells
        ),
        other => println!("sub in xqc: {:?}", other.map(|o| o.is_some()).map_err(|e| e.api_message().map(str::to_owned))),
    }
}

/// Whether Hermes takes the whisper and bell topics with the account's GQL token. Only
/// subscribes; prints Twitch's answers.
#[tokio::test]
#[ignore = "network, signed in"]
async fn phase12_hermes_topics() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let me = gql::profile::profile(&http, auth).await.unwrap();
    let url = format!("wss://hermes.twitch.tv/v1?clientId={}", cyex_twitch::clients::WEB);
    let (mut ws, _) = tokio_tungstenite::connect_async(url.as_str()).await.unwrap();
    let ts = cyex_twitch::clock::now_iso();
    let mut sent_auth = false;
    let mut subscribed = false;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20);
    while let Ok(Some(Ok(msg))) = tokio::time::timeout_at(deadline, ws.next()).await {
        let Message::Text(text) = msg else { continue };
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let kind = v["type"].as_str().unwrap_or_default().to_owned();
        if kind == "welcome" && !sent_auth {
            sent_auth = true;
            let a = serde_json::json!({"type":"authenticate","id":"a1","authenticate":{"token":token},"timestamp":ts});
            ws.send(Message::Text(a.to_string().into())).await.unwrap();
        } else if kind == "authenticateResponse" {
            println!("auth: {}", v["authenticateResponse"]);
            if !subscribed {
                subscribed = true;
                for (sid, topic) in [("w", format!("whispers.{}", me.id)), ("o", format!("onsite-notifications.{}", me.id))] {
                    let s = serde_json::json!({"type":"subscribe","id":format!("r{sid}"),
                        "subscribe":{"id":sid,"type":"pubsub","pubsub":{"topic":topic}},"timestamp":ts});
                    ws.send(Message::Text(s.to_string().into())).await.unwrap();
                }
            }
        } else if kind == "subscribeResponse" {
            println!("subscribe {}: {}", v["parentId"], v["subscribeResponse"]);
        }
    }
}

/// V101-28: the stream manager's GQL reads on the signed-in account's own channel. Reads
/// only; the dashboard's writes are schema-checked, never run.
#[tokio::test]
#[ignore = "network, signed in"]
async fn stream_manager_reads() {
    use cyex_twitch::gql::dashboard::{ads, info, modes, stats, together};
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let me = gql::profile::profile(&http, auth).await.unwrap();
    let id = me.id.as_str();
    println!("info: {:?}", info::info(&http, auth, id).await.map(|i| (i.game_name, i.broadcaster_language, i.tags.len())));
    println!("categories: {:?}", info::categories(&http, auth, "just chat").await.map(|c| c.into_iter().take(2).collect::<Vec<_>>()));
    println!("ads: {:?}", ads::schedule(&http, auth, id).await);
    println!("modes: {:?}", modes::modes(&http, auth, id).await);
    println!("together: {:?}", together::session(&http, auth, id).await);
    println!("stats: {:?}", stats::numbers_now(&http, auth, id).await.map(|n| (n.live, n.followers, n.subs)));
}

/// D1, D2, D5, D6: the feed's history, raid suggestions, ad settings, join requests and
/// auto-add on the account's own channel. Reads only.
#[tokio::test]
#[ignore = "network, signed in"]
async fn stream_manager_batch7_reads() {
    use cyex_twitch::gql::dashboard::{ads, feed, raids, together};
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let me = gql::profile::profile(&http, auth).await.unwrap();
    let id = me.id.as_str();
    println!("history: {:?}", feed::history(&http, auth, id).await.map(|n| n.len()));
    println!("picks: {:?}", raids::suggestions(&http, auth, id).await.map(|p| p.len()));
    println!("ad settings: {:?}", ads::settings(&http, auth, id).await);
    println!("join requests: {:?}", together::join_requests(&http, auth, id).await.map(|r| r.len()));
    println!("auto add: {:?}", together::auto_add(&http, auth, id, &me.login).await);
}

/// V101-28: which dashboard topics the signed-in account may follow on its own channel.
/// Subscribes only; nothing is written.
#[tokio::test]
#[ignore = "network, signed in"]
async fn stream_manager_topics() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let me = gql::profile::profile(&http, auth).await.unwrap();
    let id = me.id.as_str();
    let url = format!("wss://hermes.twitch.tv/v1?clientId={}", cyex_twitch::clients::WEB);
    let (mut ws, _) = tokio_tungstenite::connect_async(url.as_str()).await.unwrap();
    let ts = cyex_twitch::clock::now_iso();
    let topics = [
        ("f", format!("activity-feed-broadcaster-v2.{id}.{id}")),
        ("l", format!("activity-feed-alerts-v2.{id}")),
        ("s", format!("ads-manager.{id}.{id}")),
    ];
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    while let Ok(Some(Ok(msg))) = tokio::time::timeout_at(deadline, ws.next()).await {
        let Message::Text(text) = msg else { continue };
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        match v["type"].as_str().unwrap_or_default() {
            "welcome" => {
                let a = serde_json::json!({"type":"authenticate","id":"a1","authenticate":{"token":token},"timestamp":ts});
                ws.send(Message::Text(a.to_string().into())).await.unwrap();
            }
            "authenticateResponse" => {
                println!("auth: {}", v["authenticateResponse"]["result"]);
                for (sid, topic) in &topics {
                    let s = serde_json::json!({"type":"subscribe","id":format!("r{sid}"),
                        "subscribe":{"id":sid,"type":"pubsub","pubsub":{"topic":topic}},"timestamp":ts});
                    ws.send(Message::Text(s.to_string().into())).await.unwrap();
                }
            }
            "subscribeResponse" => println!("subscribe {}: {}", v["parentId"], v["subscribeResponse"]),
            _ => {}
        }
    }
}

/// V101-05: the usercard's mod history (lists, ban status) on the account's own channel,
/// and a profile banner. Reads only.
#[tokio::test]
#[ignore = "network, signed in"]
async fn usercard_mod_history() {
    let http = Http::new().unwrap();
    let (token, client_id) = gql_sign_in();
    let auth = Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let me = gql::profile::profile(&http, auth).await.unwrap();
    for target in [me.id.as_str(), "12826"] {
        let h = gql::modlogs::history(&http, auth, &me.id, target).await.unwrap();
        println!(
            "{target}: msgs {} bans {} timeouts {} warns {} actions {} ban {:?}",
            h.message_count,
            h.bans,
            h.timeouts,
            h.warnings,
            h.actions.len(),
            h.ban
        );
    }
    let p = gql::profile(&http, Some("twitch"), None).await.unwrap().unwrap();
    println!("banner: {}", p.banner.is_some());
}
