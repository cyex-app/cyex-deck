//! Plugins end to end: a real Lua thread against a fake app.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use serde_json::{Map, json};

use crate::host::*;
use crate::manifest::DATA;
use crate::perm::Granted;
use crate::runtime::{Config, Event, Plugin, Reply};

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
    assert!(err.contains("can't be a command"), "{err}");
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
    assert_eq!(p.exports().menu, ["Count words"]);
    assert_eq!(ask(|r| p.menu(0, "open", Some(line("open", "abc")), r)), Ok(()));
    assert_eq!(repl(&p, "words"), Ok("\"3 in open\"".into()));
    assert_eq!(repl(&p, "cyex.current_account().login"), Ok("\"me\"".into()));
    // Registering after start tells the app.
    let before = fake.changed.load(Ordering::Relaxed);
    repl(&p, "cyex.register_command('later', function() end)").unwrap();
    assert_eq!(fake.changed.load(Ordering::Relaxed), before + 1);
}
