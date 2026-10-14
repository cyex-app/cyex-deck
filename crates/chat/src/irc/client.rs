//! One IRC connection for one identity, an account or anonymous (1.8). Logs in, joins
//! under the rate limit, answers PINGs, notices dead links and reconnects with backoff.
//! Lines go out as events; rooms parse them. Tells the app which JOINs went out (V105-05),
//! so it can tell one Twitch never confirms.

use std::collections::{BTreeSet, VecDeque};
use std::future::{Future, pending};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, WriteHalf};
use tokio::sync::mpsc;
use tokio::time::{Instant, interval, sleep, timeout};

use super::limiter::Window;
use super::line::Line;
use super::write;

const HOST: &str = "irc.chat.twitch.tv";
const PORT: u16 = 6697;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10);
/// Send our own PING after this long without any line.
const IDLE_PING: Duration = Duration::from_secs(60);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// A session that lasted this long resets the backoff.
const STABLE: Duration = Duration::from_secs(60);
/// Keeps JOIN lines well under IRC's 512 bytes.
const MAX_JOIN_LINE: usize = 400;

pub trait Stream: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> Stream for T {}

pub type BoxStream = Box<dyn Stream>;
pub type Connect =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = io::Result<BoxStream>> + Send>> + Send + Sync>;

/// IRC login for an account.
pub struct Login {
    pub nick: String,
    pub token: String,
}

/// Asked on every (re)connect, so refreshed tokens are picked up.
pub trait Credentials: Send + Sync + 'static {
    /// `None` reads anonymously.
    fn login(&self) -> Option<Login>;

    /// C37 Ask for joins and leaves too.
    fn membership(&self) -> bool {
        false
    }
}

/// Anonymous reading.
pub struct Anonymous;

impl Credentials for Anonymous {
    fn login(&self) -> Option<Login> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    Connecting,
    Connected,
    Reconnecting,
    AuthFailed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Status(LinkStatus),
    Line(String),
    /// V105-05 JOIN lines just went out for these channels. Twitch confirms each with our own JOIN.
    Joining(Vec<String>),
}

enum Cmd {
    Join(String),
    Part(String),
    Send(String),
    Reconnect,
    Stop,
}

/// Handle to the connection task. Dropping every handle stops it.
#[derive(Clone)]
pub struct Client {
    tx: mpsc::UnboundedSender<Cmd>,
}

impl Client {
    /// Returns the handle and the task to spawn.
    pub fn new(
        creds: Arc<dyn Credentials>,
        connect: Connect,
        events: mpsc::UnboundedSender<Event>,
    ) -> (Self, impl Future<Output = ()> + Send + 'static) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, run(creds, connect, rx, events))
    }

    pub fn join(&self, channel: &str) {
        let _ = self.tx.send(Cmd::Join(channel.to_ascii_lowercase()));
    }

    pub fn part(&self, channel: &str) {
        let _ = self.tx.send(Cmd::Part(channel.to_ascii_lowercase()));
    }

    /// A raw line, e.g. from [`write::privmsg`]. Dropped while disconnected.
    pub fn send(&self, line: String) {
        let _ = self.tx.send(Cmd::Send(line));
    }

    /// Reconnects now, e.g. with a new token after an auth failure.
    pub fn reconnect(&self) {
        let _ = self.tx.send(Cmd::Reconnect);
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

/// TLS to Twitch through Windows SChannel.
pub fn tls() -> Connect {
    Arc::new(|| {
        Box::pin(async {
            let tcp = tokio::net::TcpStream::connect((HOST, PORT)).await?;
            tcp.set_nodelay(true)?;
            let tls = tokio_native_tls::native_tls::TlsConnector::new().map_err(io::Error::other)?;
            let stream = tokio_native_tls::TlsConnector::from(tls)
                .connect(HOST, tcp)
                .await
                .map_err(io::Error::other)?;
            Ok(Box::new(stream) as BoxStream)
        })
    })
}

enum End {
    Stop,
    AuthFailed,
    Lost { now: bool },
}

async fn run(
    creds: Arc<dyn Credentials>,
    connect: Connect,
    mut rx: mpsc::UnboundedReceiver<Cmd>,
    events: mpsc::UnboundedSender<Event>,
) {
    let mut channels = BTreeSet::new();
    let mut backoff = Duration::ZERO;
    loop {
        let _ = events.send(Event::Status(LinkStatus::Connecting));
        let started = Instant::now();
        let delay = match session(creds.as_ref(), &connect, &mut channels, &mut rx, &events).await {
            End::Stop => return,
            End::AuthFailed => {
                let _ = events.send(Event::Status(LinkStatus::AuthFailed));
                backoff = Duration::ZERO;
                None
            }
            End::Lost { now } => {
                let _ = events.send(Event::Status(LinkStatus::Reconnecting));
                backoff = if now || started.elapsed() >= STABLE {
                    Duration::ZERO
                } else {
                    (backoff * 2).clamp(Duration::from_secs(1), MAX_BACKOFF)
                };
                Some(backoff)
            }
        };
        if !idle(&mut rx, &mut channels, delay).await {
            return;
        }
    }
}

/// Waits between sessions while keeping track of joins and parts. `None` waits for an
/// explicit reconnect. Returns false when the client stops.
async fn idle(rx: &mut mpsc::UnboundedReceiver<Cmd>, channels: &mut BTreeSet<String>, delay: Option<Duration>) -> bool {
    let wait = async {
        match delay {
            Some(d) => sleep(d).await,
            None => pending().await,
        }
    };
    tokio::pin!(wait);
    loop {
        tokio::select! {
            () = &mut wait => return true,
            cmd = rx.recv() => match cmd {
                None | Some(Cmd::Stop) => return false,
                Some(Cmd::Join(c)) => { channels.insert(c); }
                Some(Cmd::Part(c)) => { channels.remove(&c); }
                Some(Cmd::Reconnect) => return true,
                Some(Cmd::Send(_)) => {}
            },
        }
    }
}

async fn session(
    creds: &dyn Credentials,
    connect: &Connect,
    channels: &mut BTreeSet<String>,
    rx: &mut mpsc::UnboundedReceiver<Cmd>,
    events: &mpsc::UnboundedSender<Event>,
) -> End {
    let lost = End::Lost { now: false };
    let Ok(Ok(stream)) = timeout(CONNECT_TIMEOUT, connect()).await else {
        return lost;
    };
    let (reader, writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();
    let mut out = Writer(writer);

    let mut hello = vec![write::CAPS.to_owned()];
    if creds.membership() {
        hello.push(write::MEMBERSHIP.to_owned());
    }
    match creds.login() {
        Some(l) => {
            hello.push(format!("PASS oauth:{}", l.token));
            hello.push(format!("NICK {}", l.nick.to_ascii_lowercase()));
        }
        None => hello.push(format!("NICK justinfan{}", anon_number())),
    }
    if out.lines(&hello).await.is_err() {
        return lost;
    }

    // Wait for the welcome (001) or a refusal.
    let deadline = sleep(LOGIN_TIMEOUT);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            () = &mut deadline => return lost,
            line = lines.next_line() => {
                let Ok(Some(raw)) = line else { return lost };
                let Some(l) = Line::parse(&raw) else { continue };
                match l.command {
                    "001" => break,
                    "PING" => { let _ = out.line(&pong(&l)).await; }
                    "NOTICE" if refused(l.trailing().unwrap_or("")) => return End::AuthFailed,
                    _ => {}
                }
            }
            cmd = rx.recv() => match cmd {
                None | Some(Cmd::Stop) => return End::Stop,
                Some(Cmd::Join(c)) => { channels.insert(c); }
                Some(Cmd::Part(c)) => { channels.remove(&c); }
                Some(Cmd::Reconnect) => return End::Lost { now: true },
                Some(Cmd::Send(_)) => {}
            },
        }
    }
    let _ = events.send(Event::Status(LinkStatus::Connected));

    let mut joins = Window::joins();
    let mut queue: VecDeque<String> = channels.iter().cloned().collect();
    let mut last_rx = Instant::now();
    let mut ping_sent: Option<Instant> = None;
    let mut tick = interval(Duration::from_secs(5));

    loop {
        match out.flush_joins(&mut queue, &mut joins).await {
            Err(_) => return lost,
            Ok(sent) if !sent.is_empty() => {
                let _ = events.send(Event::Joining(sent));
            }
            Ok(_) => {}
        }
        let join_wait = (!queue.is_empty())
            .then(|| joins.wait(Window::JOIN_LIMIT, Instant::now()))
            .flatten();

        tokio::select! {
            line = lines.next_line() => {
                let Ok(Some(raw)) = line else { return lost };
                last_rx = Instant::now();
                ping_sent = None;
                let Some(l) = Line::parse(&raw) else { continue };
                match l.command {
                    "PING" => if out.line(&pong(&l)).await.is_err() { return lost },
                    "PONG" => {}
                    "RECONNECT" => return End::Lost { now: true },
                    _ => { let _ = events.send(Event::Line(raw)); }
                }
            }
            cmd = rx.recv() => {
                let sent = match cmd {
                    None | Some(Cmd::Stop) => return End::Stop,
                    Some(Cmd::Reconnect) => return End::Lost { now: true },
                    Some(Cmd::Join(c)) => {
                        if channels.insert(c.clone()) {
                            queue.push_back(c);
                        }
                        Ok(())
                    }
                    Some(Cmd::Part(c)) => {
                        queue.retain(|q| *q != c);
                        if channels.remove(&c) { out.line(&write::part(&c)).await } else { Ok(()) }
                    }
                    Some(Cmd::Send(line)) => out.line(&line).await,
                };
                if sent.is_err() {
                    return lost;
                }
            }
            () = sleep(join_wait.unwrap_or_default()), if join_wait.is_some() => {}
            _ = tick.tick() => {
                match ping_sent {
                    Some(at) if at.elapsed() >= PONG_TIMEOUT => return lost,
                    None if last_rx.elapsed() >= IDLE_PING => {
                        if out.line("PING :tmi.twitch.tv").await.is_err() {
                            return lost;
                        }
                        ping_sent = Some(Instant::now());
                    }
                    _ => {}
                }
            }
        }
    }
}

struct Writer(WriteHalf<BoxStream>);

impl Writer {
    async fn line(&mut self, line: &str) -> io::Result<()> {
        self.0.write_all(format!("{line}\r\n").as_bytes()).await?;
        self.0.flush().await
    }

    async fn lines(&mut self, lines: &[String]) -> io::Result<()> {
        for l in lines {
            self.line(l).await?;
        }
        Ok(())
    }

    /// Sends as many queued JOINs as the rate limit allows, a few per line.
    async fn flush_joins(&mut self, queue: &mut VecDeque<String>, limit: &mut Window) -> io::Result<Vec<String>> {
        let mut batch: Vec<String> = Vec::new();
        let mut sent: Vec<String> = Vec::new();
        let mut len = 0;
        while let Some(next) = queue.front() {
            if len + next.len() + 2 > MAX_JOIN_LINE && !batch.is_empty() {
                self.line(&write::join(&batch)).await?;
                sent.append(&mut batch);
                len = 0;
                continue;
            }
            if !limit.try_take(Window::JOIN_LIMIT, Instant::now()) {
                break;
            }
            len += next.len() + 2;
            batch.extend(queue.pop_front());
        }
        if !batch.is_empty() {
            self.line(&write::join(&batch)).await?;
            sent.append(&mut batch);
        }
        Ok(sent)
    }
}

fn pong(ping: &Line<'_>) -> String {
    format!("PONG :{}", ping.trailing().unwrap_or("tmi.twitch.tv"))
}

/// Twitch's answers to a bad or expired token.
fn refused(notice: &str) -> bool {
    let n = notice.to_ascii_lowercase();
    n.contains("authentication failed") || n.contains("improperly formatted auth") || n.contains("invalid nick")
}

fn anon_number() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    10_000 + nanos % 90_000
}

#[cfg(test)]
mod tests {
    use tokio::io::{DuplexStream, ReadHalf, duplex};
    use tokio::sync::Mutex;

    use super::*;

    /// The fake server's side of each connection the client opens.
    type Servers = Arc<Mutex<mpsc::UnboundedReceiver<DuplexStream>>>;

    fn fake() -> (Connect, Servers) {
        let (tx, rx) = mpsc::unbounded_channel();
        let connect: Connect = Arc::new(move || {
            let (client, server) = duplex(64 * 1024);
            let _ = tx.send(server);
            Box::pin(async move { Ok(Box::new(client) as BoxStream) })
        });
        (connect, Arc::new(Mutex::new(rx)))
    }

    struct Server {
        lines: tokio::io::Lines<BufReader<ReadHalf<DuplexStream>>>,
        out: WriteHalf<DuplexStream>,
    }

    impl Server {
        async fn accept(servers: &Servers) -> Self {
            let s = servers.lock().await.recv().await.unwrap();
            let (r, w) = tokio::io::split(s);
            Self {
                lines: BufReader::new(r).lines(),
                out: w,
            }
        }

        async fn read(&mut self) -> String {
            self.lines.next_line().await.unwrap().unwrap()
        }

        async fn say(&mut self, line: &str) {
            self.out.write_all(format!("{line}\r\n").as_bytes()).await.unwrap();
        }
    }

    struct Token;

    impl Credentials for Token {
        fn login(&self) -> Option<Login> {
            Some(Login {
                nick: "Me".into(),
                token: "abc".into(),
            })
        }
    }

    fn start(creds: Arc<dyn Credentials>) -> (Client, mpsc::UnboundedReceiver<Event>, Servers) {
        let (connect, servers) = fake();
        let (tx, rx) = mpsc::unbounded_channel();
        let (client, task) = Client::new(creds, connect, tx);
        tokio::spawn(task);
        (client, rx, servers)
    }

    async fn next_line(events: &mut mpsc::UnboundedReceiver<Event>) -> String {
        loop {
            if let Event::Line(l) = events.recv().await.unwrap() {
                return l;
            }
        }
    }

    #[tokio::test]
    async fn anonymous_login_join_and_lines() {
        let (client, mut events, servers) = start(Arc::new(Anonymous));
        client.join("Foo");
        let mut s = Server::accept(&servers).await;
        assert_eq!(s.read().await, write::CAPS);
        assert!(s.read().await.starts_with("NICK justinfan"));
        s.say(":tmi.twitch.tv 001 justinfan1 :Welcome").await;
        assert_eq!(s.read().await, "JOIN #foo");
        assert_eq!(events.recv().await, Some(Event::Status(LinkStatus::Connecting)));
        assert_eq!(events.recv().await, Some(Event::Status(LinkStatus::Connected)));
        assert_eq!(events.recv().await, Some(Event::Joining(vec!["foo".into()])));

        s.say("PING :tmi.twitch.tv").await;
        assert_eq!(s.read().await, "PONG :tmi.twitch.tv");
        s.say(":a!a@a PRIVMSG #foo :hi").await;
        assert_eq!(next_line(&mut events).await, ":a!a@a PRIVMSG #foo :hi");

        client.send("PRIVMSG #foo :yo".into());
        assert_eq!(s.read().await, "PRIVMSG #foo :yo");
        client.part("foo");
        assert_eq!(s.read().await, "PART #foo");
    }

    #[tokio::test]
    async fn token_login_and_refusal() {
        let (client, mut events, servers) = start(Arc::new(Token));
        let mut s = Server::accept(&servers).await;
        assert_eq!(s.read().await, write::CAPS);
        assert_eq!(s.read().await, "PASS oauth:abc");
        assert_eq!(s.read().await, "NICK me");
        s.say(":tmi.twitch.tv NOTICE * :Login authentication failed").await;
        assert_eq!(events.recv().await, Some(Event::Status(LinkStatus::Connecting)));
        assert_eq!(events.recv().await, Some(Event::Status(LinkStatus::AuthFailed)));

        // Waits for an explicit reconnect, then tries again.
        client.reconnect();
        let mut s = Server::accept(&servers).await;
        assert_eq!(s.read().await, write::CAPS);
    }

    #[tokio::test(start_paused = true)]
    async fn joins_respect_the_rate_limit_and_survive_reconnects() {
        let (client, mut events, servers) = start(Arc::new(Anonymous));
        for i in 0..25 {
            client.join(&format!("c{i:02}"));
        }
        let mut s = Server::accept(&servers).await;
        s.read().await;
        s.read().await;
        s.say(":tmi.twitch.tv 001 x :hi").await;
        let mut joined = 0;
        while joined < Window::JOIN_LIMIT {
            joined += s.read().await.matches('#').count();
        }
        assert_eq!(joined, Window::JOIN_LIMIT);
        // V105-05 The client reported exactly those JOINs.
        let mut reported = 0;
        while let Ok(ev) = events.try_recv() {
            if let Event::Joining(sent) = ev {
                reported += sent.len();
            }
        }
        assert_eq!(reported, Window::JOIN_LIMIT);
        // The rest follow once the window moves on.
        let rest = s.read().await;
        assert_eq!(rest.matches('#').count(), 25 - Window::JOIN_LIMIT);

        s.say(":tmi.twitch.tv RECONNECT").await;
        let mut s = Server::accept(&servers).await;
        s.read().await;
        s.read().await;
        s.say(":tmi.twitch.tv 001 x :hi").await;
        let mut joined = 0;
        while joined < Window::JOIN_LIMIT {
            joined += s.read().await.matches('#').count();
        }
        while events.try_recv().is_ok() {}
        client.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn silent_link_is_pinged_then_dropped() {
        let (_client, mut events, servers) = start(Arc::new(Anonymous));
        let mut s = Server::accept(&servers).await;
        s.read().await;
        s.read().await;
        s.say(":tmi.twitch.tv 001 x :hi").await;
        assert_eq!(s.read().await, "PING :tmi.twitch.tv");
        // No PONG: the client gives up and reconnects.
        let mut statuses = Vec::new();
        while statuses.last() != Some(&LinkStatus::Reconnecting) {
            if let Some(Event::Status(st)) = events.recv().await {
                statuses.push(st);
            }
        }
        let _again = Server::accept(&servers).await;
    }
}
