//! V106-31/32 Set editing against canned replies.

use super::edit::classify_edit;
use super::tests::{client, reply, serve, FAKE};
use super::*;

fn gql_error(code: &str, status: u16, msg: &str) -> String {
    format!(
        r#"{{"data":null,"errors":[{{"message":"{msg}","extensions":{{"code":"{code}","status":{status},"message":"{msg}"}}}}]}}"#
    )
}

fn ok_add() -> String {
    r#"{"data":{"emoteSets":{"emoteSet":{"addEmote":{"id":"S1"}}}}}"#.into()
}

async fn add_with(body: &str) -> Result<(), EditError> {
    let (base, _h) = serve(vec![reply("200 OK", body)]);
    client(&base).add_emote(FAKE, "S1", "E1", "pepeX").await
}

#[tokio::test]
async fn add_sends_bearer_op_and_alias() {
    let (base, h) = serve(vec![reply("200 OK", &ok_add())]);
    client(&base).add_emote(FAKE, "S1", "E1", "pepeX").await.unwrap();
    let req = h.join().unwrap().remove(0);
    let low = req.to_lowercase();
    assert!(low.contains(&format!("authorization: bearer {FAKE}").to_lowercase()));
    assert!(req.contains("AddEmoteToSet"));
    assert!(req.contains(r#""alias":"pepeX""#));
}

#[tokio::test]
async fn error_rows_map_to_kinds() {
    let rows = [
        (
            "LOAD_ERROR",
            400,
            "LOAD_ERROR emote set is at capacity",
            EditError::Full,
        ),
        (
            "BAD_REQUEST",
            409,
            "this emote has a conflicting name",
            EditError::NameTaken,
        ),
        ("BAD_REQUEST", 409, "emote name conflict", EditError::NameTaken),
        ("BAD_REQUEST", 400, "emote is private", EditError::Private),
        ("BAD_REQUEST", 404, "emote not found in set", EditError::NotFound),
        ("LACKING_PRIVILEGES", 403, "no", EditError::NotAllowed),
        ("LOGIN_REQUIRED", 401, "no", EditError::SignInAgain),
        ("TRANSACTION_ERROR", 500, "x", EditError::Down),
        (
            "BAD_REQUEST",
            400,
            "emote is not allowed in personal emote sets",
            EditError::NotAllowed,
        ),
        ("WHAT", 400, "huh", EditError::Bad),
    ];
    for (code, status, msg, want) in rows {
        assert_eq!(
            add_with(&gql_error(code, status, msg)).await,
            Err(want),
            "{code} {msg}"
        );
    }
    let no_code = r#"{"data":null,"errors":[{"message":"Failed to parse \"String\": invalid emote alias"}]}"#;
    assert_eq!(add_with(no_code).await, Err(EditError::BadName));
}

#[tokio::test]
async fn rate_limit_waits() {
    let with_header = r#"{"data":null,"errors":[{"message":"x","extensions":{"code":"RATE_LIMIT_EXCEEDED","headers":{"x-ratelimit-emote_set_change-reset":"12"}}}]}"#;
    assert_eq!(
        add_with(with_header).await,
        Err(EditError::RateLimited { seconds: 12 })
    );
    let bare = r#"{"data":null,"errors":[{"message":"x","extensions":{"code":"RATE_LIMIT_EXCEEDED"}}]}"#;
    let (base, _h) = serve(vec![format!(
        "HTTP/1.1 200 OK\r\nretry-after: 7\r\nx-ratelimit-reset: 7\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{bare}",
        bare.len()
    )
    .into_bytes()]);
    let got = client(&base).add_emote(FAKE, "S1", "E1", "a").await;
    assert!(matches!(got, Err(EditError::RateLimited { .. })));
    // Pure: header hint in seconds, unix time falls back.
    let body = bare.as_bytes();
    assert_eq!(
        classify_edit(200, Some(7), body),
        Err(EditError::RateLimited { seconds: 7 })
    );
    assert_eq!(
        classify_edit(200, Some(1_760_000_000), body),
        Err(EditError::RateLimited { seconds: 30 })
    );
}

#[tokio::test]
async fn http_and_shape_failures() {
    let (base, _h) = serve(vec![reply("401 Unauthorized", "{}")]);
    assert_eq!(
        client(&base).add_emote(FAKE, "S1", "E1", "a").await,
        Err(EditError::SignInAgain)
    );
    // Closed port.
    let dead = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", l.local_addr().unwrap())
    };
    assert_eq!(
        client(&dead).add_emote(FAKE, "S1", "E1", "a").await,
        Err(EditError::Down)
    );
    let null = r#"{"data":{"emoteSets":{"emoteSet":{"addEmote":null}}}}"#;
    assert_eq!(add_with(null).await, Err(EditError::Bad));
}

#[tokio::test]
async fn remove_and_rename_send_their_documents() {
    let ok_rm = r#"{"data":{"emoteSets":{"emoteSet":{"removeEmote":{"id":"S1"}}}}}"#;
    let ok_rn = r#"{"data":{"emoteSets":{"emoteSet":{"updateEmoteAlias":{"id":"S1"}}}}}"#;
    let (base, h) = serve(vec![reply("200 OK", ok_rm), reply("200 OK", ok_rn)]);
    let c = client(&base);
    c.remove_emote(FAKE, "S1", "E1", "a").await.unwrap();
    c.rename_emote(FAKE, "S1", "E1", "a", "b").await.unwrap();
    let seen = h.join().unwrap();
    assert!(seen[0].contains("RemoveEmoteFromSet"));
    assert!(seen[1].contains("RenameEmoteInSet"));
    assert!(seen[1].contains(r#""alias":"b""#));
}

#[tokio::test]
async fn reach_parses_grants() {
    let body = r#"{"data":{"users":{"userByConnection":{"id":"U0",
      "mainConnection":{"platformDisplayName":"Me"},"style":{"activeEmoteSetId":"S0"},
      "editorFor":[
        {"state":"ACCEPTED","permissions":{"emoteSet":{"manage":true}},"user":{"id":"U1","mainConnection":{"platformDisplayName":"A"},"style":{"activeEmoteSetId":"S1"}}},
        {"state":"PENDING","permissions":{"emoteSet":{"manage":true}},"user":{"id":"U2","mainConnection":null,"style":null}},
        {"state":"ACCEPTED","permissions":{"emoteSet":{"manage":false}},"user":{"id":"U3"}},
        {"state":"ACCEPTED","permissions":{"emoteSet":{"manage":true}},"user":null}
      ]}}}}"#;
    let (base, h) = serve(vec![reply("200 OK", body)]);
    let r = client(&base).reach("123").await.unwrap().unwrap();
    let req = h.join().unwrap().remove(0).to_lowercase();
    assert!(!req.contains("authorization"));
    assert!(req.contains("x-ignore-auth-failure: true"));
    assert_eq!(r.user_id, "U0");
    assert_eq!(r.name.as_deref(), Some("Me"));
    assert_eq!(r.set_id.as_deref(), Some("S0"));
    assert_eq!(r.grants.len(), 3);
    assert!(r.grants[0].accepted && r.grants[0].manage);
    assert_eq!(r.grants[0].set_id.as_deref(), Some("S1"));
    assert!(!r.grants[1].accepted && r.grants[1].name.is_none());
    assert!(r.grants[2].accepted && !r.grants[2].manage);

    let (base, _h) = serve(vec![reply(
        "200 OK",
        r#"{"data":{"users":{"userByConnection":null}}}"#,
    )]);
    assert!(client(&base).reach("1").await.unwrap().is_none());
}

#[tokio::test]
async fn edit_sets_maps_entries_and_caps_ids() {
    let body = r#"{"data":{"emoteSets":{"emoteSets":[
      {"id":"S1","kind":"NORMAL","capacity":1000,"ownerId":"U1","emotes":{"items":[
        {"id":"E1","alias":"a","emote":{"id":"E1"}},{"id":"E2","alias":"dead","emote":null}]}},
      {"id":"S2","kind":"PERSONAL","capacity":null,"ownerId":null,"emotes":{"items":[]}},
      {"id":"S3","kind":"GLOBAL","capacity":-1,"emotes":{"items":[]}}]}}}"#;
    let (base, h) = serve(vec![reply("200 OK", body)]);
    let ids: Vec<String> = (0..60).map(|i| format!("S{i}")).collect();
    let sets = client(&base).edit_sets(&ids).await.unwrap();
    let req = h.join().unwrap().remove(0);
    assert!(req.contains("\"S49\"") && !req.contains("\"S50\""));
    assert_eq!(sets[0].kind, sets::SetKind::Normal);
    assert_eq!(sets[0].capacity, Some(1000));
    assert_eq!(sets[0].entries, [("E1".to_owned(), "a".to_owned())]);
    assert_eq!(sets[1].kind, sets::SetKind::Personal);
    assert_eq!(sets[1].capacity, None);
    assert_eq!(sets[2].kind, sets::SetKind::Other);
    assert_eq!(sets[2].capacity, None);
}

#[tokio::test]
async fn emote_brief_reads_one() {
    let body = r#"{"data":{"emotes":{"emote":{"id":"E1","defaultName":"pepeD","ownerId":"U1","flags":{"private":true}}}}}"#;
    let (base, h) = serve(vec![reply("200 OK", body)]);
    let b = client(&base).emote_brief("E1").await.unwrap().unwrap();
    let req = h.join().unwrap().remove(0).to_lowercase();
    assert!(!req.contains("authorization"));
    assert_eq!(b.name, "pepeD");
    assert!(b.private);
    assert_eq!(b.owner_id.as_deref(), Some("U1"));

    let (base, _h) = serve(vec![reply("200 OK", r#"{"data":{"emotes":{"emote":null}}}"#)]);
    assert!(client(&base).emote_brief("X").await.unwrap().is_none());
}
