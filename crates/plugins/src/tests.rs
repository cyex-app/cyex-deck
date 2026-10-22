//! Plugins end to end: a real Lua thread against a fake app.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use serde_json::{Map, json};

use crate::host::*;
use crate::manifest::DATA;
use crate::perm::Granted;
use crate::runtime::{Config, Event, MenuPlace, Plugin, Reply};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A fresh folder under the system's temp folder.
pub fn temp_dir() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("cyex-plugins-test-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[derive(Default)]
struct Fake {
    logs: Mutex<Vec<(Level, String)>>,
    sent: Mutex<Vec<(String, String)>>,
    system: Mutex<Vec<(String, String)>>,
    requests: Mutex<Vec<HttpRequest>>,
    changed: AtomicUsize,
    /// S6 Sockets the plugin opened: what it asked for, what it sends, and its events.
    sockets: Mutex<Vec<(SocketRequest, tokio::sync::mpsc::Receiver<SocketOut>, Emit)>>,
    /// S7 Lines added (`add`), replaced (`replace`) and changed (`change`), in order.
    lines: Mutex<Vec<(&'static str, String, NewLine)>>,
    changes: Mutex<Vec<(String, Change)>>,
    /// S13 Tabs asked for: (`open` or `select`, login or id).
    tabs: Mutex<Vec<(&'static str, String)>>,
    /// S14 Lines taken back (`remove`, id) and chats cleared (`clear`, login).
    gone: Mutex<Vec<(&'static str, String)>>,
}

impl Fake {
    fn logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().iter().map(|(_, t)| t.clone()).collect()
    }
}

impl Host for Fake {
    fn log(&self, _plugin: &str, level: Level, text: &str) {
        self.logs.lock().unwrap().push((level, text.to_owned()));
    }
    fn account(&self) -> Option<Account> {
        Some(Account {
            login: "me".into(),
            id: "1".into(),
            name: "Me".into(),
        })
    }
    fn channel(&self, login: &str) -> Option<ChannelInfo> {
        (login == "open").then(|| ChannelInfo {
            login: "open".into(),
            name: "Open".into(),
            id: "42".into(),
            moderator: true,
            modes: RoomModes {
                slow_mode: Some(30),
                ..RoomModes::default()
            },
            ..ChannelInfo::default()
        })
    }
    fn lines(&self, login: &str, n: usize) -> Vec<Line> {
        (0..n.min(3)).map(|i| line(login, &format!("old {i}"))).collect()
    }
    fn send(&self, _plugin: &str, login: &str, text: &str) -> Result<(), String> {
        self.sent.lock().unwrap().push((login.into(), text.into()));
        Ok(())
    }
    fn system(&self, _plugin: &str, login: &str, text: &str) {
        self.system.lock().unwrap().push((login.into(), text.into()));
    }
    fn http(&self, req: HttpRequest, done: Box<dyn FnOnce(HttpResponse) + Send>) {
        let status = if req.url.contains("missing") { 404 } else { 200 };
        self.requests.lock().unwrap().push(req);
        done(HttpResponse {
            status,
            data: br#"{"n": 7}"#.to_vec(),
            error: None,
        });
    }
    fn socket(&self, req: SocketRequest, out: tokio::sync::mpsc::Receiver<SocketOut>, emit: Emit) {
        self.sockets.lock().unwrap().push((req, out, emit));
    }
    fn add_line(&self, _plugin: &str, login: &str, line: NewLine) {
        self.lines.lock().unwrap().push(("add", login.into(), line));
    }
    fn replace_line(&self, _plugin: &str, login: &str, line: NewLine) {
        self.lines.lock().unwrap().push(("replace", login.into(), line));
    }
    fn change_line(&self, _plugin: &str, _login: &str, id: &str, change: Change) {
        self.changes.lock().unwrap().push((id.into(), change));
    }
    fn find_line(&self, login: &str, id: &str) -> Option<Line> {
        (id == "m1").then(|| line(login, "found"))
    }
    fn layout(&self) -> Layout {
        Layout {
            active: Some("t1".into()),
            tabs: vec![LayoutTab {
                id: "t1".into(),
                name: "Open".into(),
                group: None,
                columns: vec![vec![LayoutTile {
                    id: "x".into(),
                    channel: "open".into(),
                    name: "Open".into(),
                    chat: true,
                    ..LayoutTile::default()
                }]],
            }],
        }
    }
    fn open_channel(&self, _plugin: &str, login: &str) {
        self.tabs.lock().unwrap().push(("open", login.into()));
    }
    fn select_tab(&self, _plugin: &str, id: &str) {
        self.tabs.lock().unwrap().push(("select", id.into()));
    }
    fn remove_line(&self, _plugin: &str, _login: &str, id: &str) {
        self.gone.lock().unwrap().push(("remove", id.into()));
    }
    fn clear_lines(&self, _plugin: &str, login: &str) {
        self.gone.lock().unwrap().push(("clear", login.into()));
    }
    fn reserved(&self, command: &str) -> bool {
        command == "ban"
    }
    fn exports_changed(&self, _plugin: &str) {
        self.changed.fetch_add(1, Ordering::Relaxed);
    }
}

fn line(channel: &str, text: &str) -> Line {
    Line {
        id: "m1".into(),
        channel: channel.into(),
        kind: "chat",
        text: text.into(),
        sender: Some(Sender {
            id: "9".into(),
            login: "bob".into(),
            name: "Bob".into(),
            color: None,
        }),
        badges: vec!["subscriber".into()],
        ..Line::default()
    }
}

fn start(init: &str, granted: Granted) -> Result<(Plugin, Arc<Fake>, PathBuf), String> {
    start_with(init, granted, Map::new())
}

fn start_with(
    init: &str,
    granted: Granted,
    settings: Map<String, serde_json::Value>,
) -> Result<(Plugin, Arc<Fake>, PathBuf), String> {
    let dir = temp_dir();
    std::fs::write(dir.join("init.lua"), init).unwrap();
    let fake = Arc::new(Fake::default());
    let p = Plugin::start(Config {
        id: "test".into(),
        dir: dir.clone(),
        granted,
        settings,
        host: fake.clone(),
    })?;
    Ok((p, fake, dir))
}

/// Sends something that answers through a reply and waits for it.
fn ask<T: Send + 'static>(f: impl FnOnce(Reply<T>) -> bool) -> T {
    let (tx, rx) = mpsc::channel();
    assert!(f(Box::new(move |v| {
        let _ = tx.send(v);
    })));
    rx.recv_timeout(Duration::from_secs(5))
        .expect("the plugin answered")
}

fn repl(p: &Plugin, code: &str) -> Result<String, String> {
    ask(|r| p.repl(code, r))
}

fn read_only() -> Granted {
    Granted {
        read_chat: true,
        ..Granted::default()
    }
}

#[test]
fn commands_run_and_answer() {
    let (p, fake, _) = start(
        r#"
        cyex.register_command("/greet", function(ctx)
            ctx.channel:add_system_message("hi " .. (ctx.words[2] or "?"))
            return "/me waves at " .. ctx.channel:get_name()
        end)
        "#,
        Granted::default(),
    )
    .unwrap();
    assert_eq!(p.exports().commands, ["greet"]);
    let out = ask(|r| p.command("greet", vec!["/greet".into(), "bob".into()], "open", r));
    assert_eq!(out, Ok(Some("/me waves at open".into())));
    assert_eq!(fake.system.lock().unwrap()[0], ("open".into(), "hi bob".into()));
}

#[test]
fn app_and_twitch_commands_stay_theirs() {
    let err = start(
        r#"cyex.register_command("ban", function() end)"#,
        Granted::default(),
    )
    .err()
    .unwrap();
    assert!(err.contains("belongs to the app or Twitch"), "{err}");
    let err = start(
        r#"cyex.register_command("a b", function() end)"#,
        Granted::default(),
    )
    .err()
    .unwrap();
    assert!(err.contains("can’t be a command"), "{err}");
}

#[test]
fn line_callbacks_change_lines_only_with_change_chat() {
    let script = r##"
        cyex.register_callback(cyex.EventType.Message, function(msg)
            seen = (seen or 0) + 1
            if msg.text:find("spoiler") then return { hide = true } end
            if msg.sender and msg.sender.login == "bob" and msg.flags.own == nil then
                return { text = msg.text:upper(), highlight = "#ff0000" }
            end
        end)
    "##;
    let (p, _, _) = start(
        script,
        Granted {
            change_chat: true,
            ..read_only()
        },
    )
    .unwrap();
    assert!(p.has(Event::Message));
    let c = ask(|r| p.line(line("open", "hello Kappa"), Some(r))).unwrap();
    assert_eq!(c.text.as_deref(), Some("HELLO KAPPA"));
    assert_eq!(c.highlight.as_deref(), Some("#ff0000"));
    assert!(
        ask(|r| p.line(line("open", "big spoiler"), Some(r)))
            .unwrap()
            .hide
    );

    // Read only: the callback runs, its answer counts for nothing.
    let (p, _, _) = start(script, read_only()).unwrap();
    assert!(p.line(line("open", "big spoiler"), None));
    assert_eq!(repl(&p, "seen"), Ok("1".into()));
    // No permission: it can't even register.
    let err = start(script, Granted::default()).err().unwrap();
    assert!(err.contains("needs the ReadChat"), "{err}");
}

#[test]
fn line_callbacks_cant_wait_on_chat_state() {
    let (p, fake, _) = start(
        r#"
        cyex.register_callback(cyex.EventType.Message, function(msg)
            local ch = cyex.Channel.by_name(msg.channel)
            ok, err = pcall(function() return ch:is_mod() end)
            ch:add_system_message("queued")
        end)
        "#,
        read_only(),
    )
    .unwrap();
    assert_eq!(ask(|r| p.line(line("open", "x"), Some(r))), None);
    assert_eq!(repl(&p, "ok"), Ok("false".into()));
    let err = repl(&p, "err").unwrap();
    assert!(err.contains("inside a Message callback"), "{err}");
    assert_eq!(fake.system.lock().unwrap().len(), 1);
    // Outside the callback it works.
    assert_eq!(
        repl(&p, "cyex.Channel.by_name('open'):is_mod()"),
        Ok("true".into())
    );
    assert_eq!(repl(&p, "cyex.Channel.by_name('closed')"), Ok("nil".into()));
    assert_eq!(
        repl(&p, "cyex.Channel.by_name('open'):get_room_modes().slow_mode"),
        Ok("30".into())
    );
    assert_eq!(
        repl(&p, "#cyex.Channel.by_name('open'):message_snapshot(2)"),
        Ok("2".into())
    );
}

#[test]
fn slow_line_callbacks_get_paused() {
    let (p, fake, _) = start(
        r#"cyex.register_callback(cyex.EventType.Message, function() while true do end end)"#,
        Granted {
            change_chat: true,
            ..read_only()
        },
    )
    .unwrap();
    for _ in 0..3 {
        assert_eq!(ask(|r| p.line(line("open", "x"), Some(r))), None);
    }
    assert!(p.exports().paused);
    assert!(!p.has(Event::Message));
    assert!(fake.logs().iter().any(|l| l.contains("Paused")));
    // The plugin itself still answers.
    assert_eq!(repl(&p, "1 + 1"), Ok("2".into()));
}

#[test]
fn start_errors_are_short() {
    let err = start("local x = nil + 1", Granted::default()).err().unwrap();
    assert!(err.starts_with("init.lua:1:"), "{err}");
    let err = start("while true do end", Granted::default()).err().unwrap();
    assert!(err.contains("took too long"), "{err}");
    let err = start("if then", Granted::default()).err().unwrap();
    assert!(!err.contains("traceback"), "{err}");
}

#[test]
fn sending_and_completion_callbacks() {
    let (p, _, _) = start(
        r#"
        cyex.register_callback(cyex.EventType.Sending, function(ctx)
            if ctx.text == "stop" then return false end
            if ctx.text:sub(1, 1) == "!" then return ctx.text .. " please" end
        end)
        cyex.register_callback(cyex.EventType.CompletionRequested, function(ev)
            if ev.query:sub(1, 1) == "$" then return { values = { "$one", "$two" }, hide_others = true } end
            return { values = {} }
        end)
        "#,
        Granted {
            send: true,
            ..Granted::default()
        },
    )
    .unwrap();
    assert_eq!(
        ask(|r| p.sending("open", "!so", r)),
        Sent::Text("!so please".into())
    );
    assert_eq!(ask(|r| p.sending("open", "stop", r)), Sent::Stop);
    assert_eq!(ask(|r| p.sending("open", "hi", r)), Sent::Unchanged);
    let req = CompletionRequest {
        query: "$o".into(),
        ..CompletionRequest::default()
    };
    let c = ask(|r| p.complete(req, r));
    assert_eq!(c.values, ["$one", "$two"]);
    assert!(c.hide_others);
}

#[test]
fn sending_messages_needs_permission_and_is_capped() {
    let (p, _, _) = start("", Granted::default()).unwrap();
    assert!(
        repl(&p, "cyex.Channel.by_name('open'):send_message('hi')")
            .unwrap_err()
            .contains("SendMessages")
    );
    let (p, fake, _) = start(
        "",
        Granted {
            send: true,
            ..Granted::default()
        },
    )
    .unwrap();
    let out = repl(
        &p,
        "for i = 1, 11 do cyex.Channel.by_name('open'):send_message('n' .. i) end",
    );
    assert!(out.unwrap_err().contains("up to 10 messages"));
    assert_eq!(fake.sent.lock().unwrap().len(), 10);
    // V107-Mb What its commands return shares the window.
    assert!(!p.take_send());
    let (p, _, _) = start(
        "",
        Granted {
            send: true,
            ..Granted::default()
        },
    )
    .unwrap();
    assert!((0..10).all(|_| p.take_send()));
    assert!(
        repl(&p, "cyex.Channel.by_name('open'):send_message('hi')")
            .unwrap_err()
            .contains("up to 10 messages")
    );
}

#[test]
fn timers_fire_on_the_plugin_thread() {
    let (p, _, _) = start("cyex.later(function() fired = true end, 20)", Granted::default()).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(repl(&p, "fired"), Ok("true".into()));
    assert!(repl(&p, "cyex.later(function() end, 0)").is_err());
}

#[test]
fn requests_stay_on_listed_hosts() {
    let granted = Granted {
        network: Some(vec!["api.example.com".into()]),
        ..Granted::default()
    };
    let (p, fake, _) = start(
        r#"
        local r = cyex.HTTPRequest.create("get", "https://api.example.com/n")
        r:set_header("Accept", "application/json")
        r:on_success(function(res) got = cyex.json.parse(res:data()).n; code = res:status() end)
        r:finally(function() done = true end)
        r:execute()
        local bad = cyex.HTTPRequest.create("GET", "https://api.example.com/missing")
        bad:on_error(function(res) failed = res:status() end)
        bad:execute()
        "#,
        granted,
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        repl(&p, "got, code, done, failed"),
        Ok("7\t200\ttrue\t404".into())
    );
    let req = fake.requests.lock().unwrap()[0].clone();
    assert_eq!(req.method, "GET");
    assert_eq!(
        req.headers,
        [("Accept".to_owned(), "application/json".to_owned())]
    );
    assert_eq!(req.hosts, ["api.example.com"]);
    assert!(repl(&p, "cyex.HTTPRequest.create('GET', 'https://other.com/')").is_err());
    assert!(repl(&p, "cyex.HTTPRequest.create('GET', 'http://127.0.0.1/')").is_err());
    let (p, _, _) = start("", Granted::default()).unwrap();
    assert!(
        repl(&p, "cyex.HTTPRequest.create('GET', 'https://api.example.com/')")
            .unwrap_err()
            .contains("Network")
    );
}

#[test]
fn files_stay_in_the_data_folder() {
    let granted = Granted {
        fs_read: true,
        fs_write: true,
        ..Granted::default()
    };
    let (p, _, dir) = start("", granted).unwrap();
    assert!(dir.join(DATA).is_dir());
    repl(
        &p,
        "cyex.data.write('a.txt', 'one') cyex.data.append('a.txt', ' two')",
    )
    .unwrap();
    assert_eq!(repl(&p, "cyex.data.read('a.txt')"), Ok("\"one two\"".into()));
    assert_eq!(
        std::fs::read_to_string(dir.join(DATA).join("a.txt")).unwrap(),
        "one two"
    );
    assert_eq!(repl(&p, "cyex.data.list()[1]"), Ok("\"a.txt\"".into()));
    assert_eq!(repl(&p, "cyex.data.read('none.txt')"), Ok("nil".into()));
    assert!(repl(&p, "cyex.data.write('../init.lua', 'x')").is_err());
    assert!(repl(&p, "cyex.data.read('..\\\\init.lua')").is_err());
    assert_eq!(repl(&p, "cyex.data.remove('a.txt')"), Ok("true".into()));
    let (p, _, _) = start(
        "",
        Granted {
            fs_read: true,
            ..Granted::default()
        },
    )
    .unwrap();
    assert!(
        repl(&p, "cyex.data.write('a.txt', 'x')")
            .unwrap_err()
            .contains("FilesystemWrite")
    );
}

#[test]
fn settings_are_read_and_followed() {
    let mut values = Map::new();
    values.insert("greeting".into(), json!("hey"));
    let (p, _, _) = start_with(
        r#"
        greeting = cyex.get_setting("greeting")
        cyex.register_callback(cyex.EventType.SettingsChanged, function(s) greeting = s.greeting end)
        "#,
        Granted::default(),
        values,
    )
    .unwrap();
    assert_eq!(repl(&p, "greeting"), Ok("\"hey\"".into()));
    let mut next = Map::new();
    next.insert("greeting".into(), json!("yo"));
    p.set_settings(next);
    assert_eq!(
        repl(&p, "greeting .. '/' .. cyex.get_setting('greeting')"),
        Ok("\"yo/yo\"".into())
    );
}

#[test]
fn json_keeps_lists_and_objects_apart() {
    let (p, _, _) = start("", Granted::default()).unwrap();
    assert_eq!(
        repl(
            &p,
            r#"cyex.json.stringify(cyex.json.parse('{"a":[1,2,{"b":null}],"c":{}}'))"#
        ),
        Ok(r#""{\"a\":[1,2,{\"b\":null}],\"c\":{}}""#.into())
    );
    assert_eq!(
        repl(&p, "cyex.json.stringify({ x = 1 })"),
        Ok(r#""{\"x\":1}""#.into())
    );
}

#[test]
fn print_logs_and_menu_entries() {
    let (p, fake, _) = start(
        r#"
        print("hello", 1, nil)
        cyex.log(cyex.LogLevel.Warning, "careful")
        cyex.register_menu_entry("Count words", function(ctx) words = #ctx.message.text .. " in " .. ctx.channel:get_name() end)
        "#,
        Granted::default(),
    )
    .unwrap();
    assert_eq!(fake.logs(), ["hello 1 nil", "careful"]);
    assert_eq!(p.exports().menu.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(), ["Count words"]);
    assert_eq!(ask(|r| p.menu(0, "open", (Some(line("open", "abc")), None), r)), Ok(()));
    assert_eq!(repl(&p, "words"), Ok("\"3 in open\"".into()));
    assert_eq!(repl(&p, "cyex.current_account().login"), Ok("\"me\"".into()));
    // Registering after start tells the app.
    let before = fake.changed.load(Ordering::Relaxed);
    repl(&p, "cyex.register_command('later', function() end)").unwrap();
    assert_eq!(fake.changed.load(Ordering::Relaxed), before + 1);
}

/// Waits until `f` holds, for things that happen on the plugin's thread.
fn soon(f: impl Fn() -> bool) {
    for _ in 0..100 {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("never happened");
}

#[test]
fn sockets_need_network_and_stay_on_listed_hosts() {
    let (p, _, _) = start("", Granted::default()).unwrap();
    assert!(repl(&p, "cyex.WebSocket.new('wss://feed.example.com')").unwrap_err().contains("Network"));
    let granted = Granted {
        network: Some(vec!["feed.example.com".into()]),
        ..Granted::default()
    };
    let (p, fake, _) = start("", granted).unwrap();
    assert!(repl(&p, "cyex.WebSocket.new('wss://other.com')").is_err());
    assert!(repl(&p, "cyex.WebSocket.new('ws://127.0.0.1:9')").is_err());
    assert!(repl(&p, "cyex.WebSocket.new('https://feed.example.com')").is_err());
    let bad_header = "cyex.WebSocket.new('wss://feed.example.com', { headers = { ['Sec-WebSocket-Key'] = 'x' } })";
    assert!(repl(&p, bad_header).is_err());
    // Four at once.
    let open = "s = {} for i = 1, 4 do s[i] = cyex.WebSocket.new('wss://feed.example.com/' .. i) end";
    assert_eq!(repl(&p, open), Ok(String::new()));
    assert!(repl(&p, "cyex.WebSocket.new('wss://feed.example.com/5')").unwrap_err().contains("4 open"));
    assert_eq!(fake.sockets.lock().unwrap().len(), 4);
    // One closes on the far side: room for another.
    let emit = fake.sockets.lock().unwrap().remove(0).2;
    emit(SocketEvent::Closed {
        code: Some(1000),
        reason: String::new(),
    });
    soon(|| repl(&p, "s5 = cyex.WebSocket.new('wss://feed.example.com/5')").is_ok());
}

#[test]
fn socket_events_reach_their_handlers() {
    let granted = Granted {
        network: Some(vec![]),
        ..Granted::default()
    };
    let (p, fake, _) = start(
        r#"
        got = {}
        ws = cyex.WebSocket.new("wss://feed.example.com/live", {
          headers = { Authorization = "Bearer x" },
          on_open = function() ws:send_text("hello") end,
          on_close = function(code, why) closed = tostring(code) .. ":" .. why end,
        })
        ws.on_text = function(data) got[#got + 1] = data end
        "#,
        granted,
    )
    .unwrap();
    let (req, mut out, emit) = fake.sockets.lock().unwrap().pop().unwrap();
    assert_eq!(req.url, "wss://feed.example.com/live");
    assert_eq!(req.headers, [("Authorization".to_owned(), "Bearer x".to_owned())]);
    assert!(emit(SocketEvent::Open));
    assert!(emit(SocketEvent::Text("a".into())));
    assert!(emit(SocketEvent::Text("b".into())));
    soon(|| repl(&p, "#got").as_deref() == Ok("2"));
    assert_eq!(out.try_recv(), Ok(SocketOut::Text("hello".into())));
    assert_eq!(repl(&p, "got[1] .. got[2]"), Ok("\"ab\"".into()));
    assert_eq!(repl(&p, "type(ws.on_text)"), Ok("\"function\"".into()));
    // Closing from Lua ends what it sends; the far side's close runs on_close.
    assert_eq!(repl(&p, "ws:close()"), Ok(String::new()));
    assert!(repl(&p, "ws:send_text('late')").unwrap_err().contains("closed"));
    assert_eq!(out.try_recv(), Err(tokio::sync::mpsc::error::TryRecvError::Disconnected));
    emit(SocketEvent::Closed {
        code: Some(1000),
        reason: "bye".into(),
    });
    soon(|| repl(&p, "closed").as_deref() == Ok("\"1000:bye\""));
}

#[test]
fn collected_sockets_close() {
    let granted = Granted {
        network: Some(vec![]),
        ..Granted::default()
    };
    let (p, fake, _) = start("", granted).unwrap();
    // The handler holds its own socket; it still goes once nothing else does.
    let code = "do local ws; ws = cyex.WebSocket.new('wss://feed.example.com', { on_text = function() ws:close() end }) end";
    assert_eq!(repl(&p, code), Ok(String::new()));
    assert_eq!(repl(&p, "collectgarbage(); collectgarbage()"), Ok(String::new()));
    let mut out = fake.sockets.lock().unwrap().pop().unwrap().1;
    assert_eq!(out.try_recv(), Err(tokio::sync::mpsc::error::TryRecvError::Disconnected));
}

#[test]
fn plugins_build_lines_from_pieces() {
    let granted = Granted {
        network: Some(vec!["cdn.example.com".into()]),
        ..read_only()
    };
    let (p, fake, _) = start(
        r##"
        local ch = cyex.Channel.by_name("open")
        local sun = cyex.Image.from_url("https://cdn.example.com/sun.png", 0.5, { 56, 56 })
        local msg = cyex.Message.new({
          id = "weather",
          highlight_color = "#f00",
          elements = {
            { type = "text", text = "Sunny ", color = "#00ff00", tooltip = "22 °C" },
            { type = "text", text = "Vote", style = "button", link = { type = cyex.LinkType.UserAction, value = "/weather vote" } },
            { type = "mention", display_name = "@Bob", login_name = "bob" },
            { type = "linebreak" },
            { type = "image", image = sun },
            { type = "scaling-image", images = cyex.ImageSet.new("https://cdn.example.com/a.png", "https://cdn.example.com/b.png") },
            { type = "text", text = "relay: hi", parse = true },
          },
        })
        first = ch:add_message(msg)
        second = ch:add_message(cyex.Message.new({ elements = { { type = "text", text = "x" } } }))
        "##,
        granted,
    )
    .unwrap();
    assert_eq!(repl(&p, "first"), Ok("\"p:test:weather\"".into()));
    assert_eq!(repl(&p, "second:sub(1, 7)"), Ok("\"p:test:\"".into()));
    let lines = fake.lines.lock().unwrap().clone();
    let (op, login, line) = &lines[0];
    assert_eq!((*op, login.as_str(), line.highlight.as_deref()), ("add", "open", Some("#ff0000")));
    assert_eq!(line.pieces.len(), 7);
    assert!(matches!(&line.pieces[0], Piece::Text { color: Some(PieceColor::Rgb(c)), tip: Some(_), .. } if c == "#00ff00"));
    assert!(matches!(&line.pieces[1], Piece::Text { style: Some(PieceStyle::Button), link: Some(LineLink::Run(t)), .. } if t == "/weather vote"));
    assert!(matches!(&line.pieces[3], Piece::Break));
    // A 2x picture: 56 px at 2x is 28 px on the line.
    assert!(matches!(&line.pieces[4], Piece::Image { urls, size: Some((28, 28)), .. } if urls[0].1 == 2));
    assert!(matches!(&line.pieces[5], Piece::Image { urls, .. } if urls.len() == 2 && urls[1].1 == 2));
    assert!(matches!(&line.pieces[6], Piece::Text { parse: true, .. }));
}

#[test]
fn plugin_lines_follow_the_rules() {
    let (p, fake, _) = start("ch = cyex.Channel.by_name('open')", read_only()).unwrap();
    let text = |s: &str| format!("cyex.Message.new({{ elements = {{ {s} }} }})");
    // Pictures need Network and https.
    assert!(repl(&p, "cyex.Image.from_url('https://cdn.example.com/a.png')").unwrap_err().contains("Network"));
    // Text that isn't a command would send a message.
    let say = text("{ type = 'text', text = 'x', link = { type = 'UserAction', value = 'hi all' } }");
    assert!(repl(&p, &say).unwrap_err().contains("SendMessages"));
    assert!(repl(&p, &text("{ type = 'timestamp' }")).unwrap_err().contains("isn’t in cyex"));
    assert!(repl(&p, &text("{ type = 'text', text = 'x', color = 'blue' }")).is_err());
    assert!(repl(&p, &text("{ type = 'text', text = 'x', link = { type = 'Url', value = 'file:///c' } }")).is_err());
    assert!(repl(&p, &format!("ch:add_message({})", text(&format!("{{ type = 'text', text = '{}' }}", "a".repeat(1001))))).is_err());
    // Someone else's line can't be swapped; changing any needs ChangeChat.
    let one = text("{ type = 'text', text = 'x' }");
    assert!(repl(&p, &format!("ch:replace_message('m1', {one})")).unwrap_err().contains("only lines"));
    assert!(repl(&p, "ch:change_message('m1', { hide = true })").unwrap_err().contains("ChangeChat"));
    assert_eq!(repl(&p, &format!("ch:replace_message(ch:add_message({one}), {one})")), Ok(String::new()));
    assert_eq!(fake.lines.lock().unwrap().iter().map(|l| l.0).collect::<Vec<_>>(), ["add", "replace"]);
    assert_eq!(repl(&p, "ch:find_message_by_id('m1').text"), Ok("\"found\"".into()));
    // Twenty a second.
    assert!(repl(&p, &format!("for i = 1, 25 do ch:add_message({one}) end")).unwrap_err().contains("20 lines"));
    let (p, fake, _) = start("ch = cyex.Channel.by_name('open')", Granted::all()).unwrap();
    assert_eq!(repl(&p, "ch:change_message('m1', { text = 'new', highlight = true })"), Ok(String::new()));
    let changes = fake.changes.lock().unwrap();
    assert_eq!((changes[0].0.as_str(), changes[0].1.text.as_deref()), ("m1", Some("new")));
}

#[test]
fn chat_events_reach_callbacks_and_channel_handlers() {
    let (p, _, _) = start(
        r#"
        n = 0
        cleared = 0
        cyex.register_callback(cyex.EventType.MessageAppended, function(l) n = n + 1; last = l.text end)
        cyex.register_callback(cyex.EventType.MessageDeleted, function(e) gone = e.id end)
        h = cyex.Channel.by_name("open"):on_messages_cleared(function() cleared = cleared + 1 end)
        "#,
        read_only(),
    )
    .unwrap();
    assert!(p.has(Event::MessageAppended) && p.has(Event::ChatCleared));
    let cleared = |channel: &str| ChatEvent::Cleared {
        channel: channel.into(),
    };
    assert!(p.chat(vec![
        ChatEvent::Appended(line("open", "hi")),
        ChatEvent::Deleted {
            channel: "open".into(),
            id: Some("m1".into()),
            user_id: None
        },
        cleared("open"),
        cleared("other"),
    ]));
    soon(|| repl(&p, "cleared").as_deref() == Ok("1"));
    assert_eq!(repl(&p, "n, last, gone"), Ok("1\t\"hi\"\t\"m1\"".into()));
    // Blocked handlers wait; disconnected ones are gone.
    assert_eq!(repl(&p, "h:block()"), Ok(String::new()));
    assert!(p.chat(vec![cleared("open"), ChatEvent::Appended(line("open", "again"))]));
    soon(|| repl(&p, "n").as_deref() == Ok("2"));
    assert_eq!(repl(&p, "cleared, h:is_blocked(), h:is_connected()"), Ok("1\ttrue\ttrue".into()));
    assert_eq!(repl(&p, "h:disconnect(); return h:is_connected()"), Ok("false".into()));
    assert!(!p.has(Event::ChatCleared));
}

#[test]
fn chat_events_need_read_chat_but_renames_dont() {
    let (p, _, _) = start("", Granted::default()).unwrap();
    let err = repl(&p, "cyex.register_callback(cyex.EventType.MessageAppended, function() end)").unwrap_err();
    assert!(err.contains("ReadChat"));
    assert!(repl(&p, "cyex.Channel.by_name('open'):on_message_appended(function() end)").is_err());
    let ok = "cyex.register_callback(cyex.EventType.ChannelRenamed, function() end)";
    assert_eq!(repl(&p, ok), Ok(String::new()));
    assert!(p.has(Event::ChannelRenamed));
}

#[test]
fn menu_entries_go_where_asked_and_layout_reads() {
    let (p, _, _) = start(
        r#"
        cyex.register_menu_entry("translate it.", function() end, { lines = { "chat", "action" } })
        cyex.register_menu_entry("Forecast", function() end, { where = "chat", group = "weather" })
        "#,
        Granted::default(),
    )
    .unwrap();
    let menu = p.exports().menu;
    assert_eq!((menu[0].label.as_str(), menu[0].place, menu[0].lines.len()), ("Translate it", MenuPlace::Message, 2));
    assert_eq!((menu[1].place, menu[1].group.as_deref()), (MenuPlace::Chat, Some("Weather")));
    assert!(repl(&p, "cyex.register_menu_entry('x', function() end, { where = 'tab' })").is_err());
    assert!(repl(&p, "cyex.register_menu_entry('x', function() end, { lines = { 'whisper' } })").is_err());
    let layout = "local l = cyex.get_layout() return l.active, l.tabs[1].columns[1][1].channel, l.tabs[1].columns[1][1].chat";
    assert_eq!(repl(&p, layout), Ok("\"t1\"\t\"open\"\ttrue".into()));
}

#[test]
fn name_entries_get_who_was_clicked() {
    let (p, _, _) = start(
        r#"cyex.register_menu_entry("Look up", function(ctx) who = ctx.user.login .. " " .. tostring(ctx.user.id) .. " " .. tostring(ctx.message) end, { where = "name" })"#,
        Granted::default(),
    )
    .unwrap();
    assert_eq!(p.exports().menu[0].place, MenuPlace::Name);
    let user = NameUser {
        login: "bob".into(),
        ..NameUser::default()
    };
    assert_eq!(ask(|r| p.menu(0, "open", (None, Some(user)), r)), Ok(()));
    assert_eq!(repl(&p, "who"), Ok("\"bob nil nil\"".into()));
}

#[test]
fn live_and_layout_events_need_no_permission() {
    let (p, _, _) = start(
        r#"
        cyex.register_callback(cyex.EventType.LiveChanged, function(e) live = e.channel .. " " .. tostring(e.live) .. " " .. (e.title or "-") end)
        cyex.register_callback(cyex.EventType.LayoutChanged, function(l) tabs = #l.tabs; active = l.active end)
        "#,
        Granted::default(),
    )
    .unwrap();
    assert!(p.has(Event::LiveChanged) && p.has(Event::LayoutChanged));
    assert!(p.chat(vec![
        ChatEvent::Live(LiveChange {
            channel: "open".into(),
            name: "Open".into(),
            live: true,
            title: Some("hi".into()),
            game: None,
        }),
        ChatEvent::Layout(Layout {
            active: Some("t2".into()),
            tabs: vec![LayoutTab::default(), LayoutTab::default()],
        }),
    ]));
    soon(|| repl(&p, "tabs").as_deref() == Ok("2"));
    assert_eq!(repl(&p, "live, active"), Ok("\"open true hi\"	\"t2\"".into()));
}

#[test]
fn tabs_need_the_tabs_permission_and_stay_in_a_limit() {
    let (p, _, _) = start("", Granted::default()).unwrap();
    assert!(repl(&p, "cyex.open_channel('xqc')").unwrap_err().contains("Tabs permission"));
    let (p, fake, _) = start(
        "",
        Granted {
            tabs: true,
            ..Granted::default()
        },
    )
    .unwrap();
    assert_eq!(repl(&p, "cyex.open_channel(' #XQC ')"), Ok(String::new()));
    assert_eq!(repl(&p, "cyex.select_tab('t1')"), Ok(String::new()));
    assert!(repl(&p, "cyex.open_channel('not a login')").unwrap_err().contains("isn’t a Twitch login"));
    assert_eq!(*fake.tabs.lock().unwrap(), [("open", "xqc".to_owned()), ("select", "t1".to_owned())]);
    let err = repl(&p, "for i = 1, 20 do cyex.select_tab('t1') end").unwrap_err();
    assert!(err.contains("up to 10 tab switches a minute"), "{err}");
    assert_eq!(fake.tabs.lock().unwrap().len(), 10);
}

#[test]
fn plugins_take_back_their_lines_and_clear_only_with_change_chat() {
    let (p, fake, _) = start("ch = cyex.Channel.by_name('open')", Granted::default()).unwrap();
    assert_eq!(repl(&p, "ch:remove_message('p:test:1')"), Ok(String::new()));
    assert!(repl(&p, "ch:remove_message('p:other:1')").unwrap_err().contains("only lines this plugin added"));
    assert!(repl(&p, "ch:clear_messages()").unwrap_err().contains("ChangeChat"));
    let (p, fake2, _) = start(
        "ch = cyex.Channel.by_name('open')",
        Granted {
            change_chat: true,
            ..read_only()
        },
    )
    .unwrap();
    assert_eq!(repl(&p, "ch:clear_messages()"), Ok(String::new()));
    assert_eq!(*fake.gone.lock().unwrap(), [("remove", "p:test:1".to_owned())]);
    assert_eq!(*fake2.gone.lock().unwrap(), [("clear", "open".to_owned())]);
}
