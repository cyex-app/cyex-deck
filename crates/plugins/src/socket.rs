//! S6 Runs a plugin's WebSocket. The address passed the plugin's hosts already; here its
//! name is looked up and only a public address is dialed, so a name can't point the socket
//! at this PC or the local network. No redirects. Messages up to 1 MB each way.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::USER_AGENT;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::host::{Emit, SocketEvent, SocketOut, SocketRequest};
use crate::perm::public;

/// Biggest message either way.
pub const MAX_MESSAGE: usize = 1 << 20;
const CONNECT: Duration = Duration::from_secs(15);

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn run(req: SocketRequest, out: Receiver<SocketOut>, emit: Emit) {
    run_with(req, out, emit, false).await;
}

/// `local`: tests may dial this PC.
pub(crate) async fn run_with(req: SocketRequest, mut out: Receiver<SocketOut>, emit: Emit, local: bool) {
    let closed = |code: Option<u16>, reason: &str| {
        emit(SocketEvent::Closed {
            code,
            reason: reason.to_owned(),
        });
    };
    let ws = match tokio::time::timeout(CONNECT, connect(&req, local)).await {
        Ok(Ok(ws)) => ws,
        Ok(Err(e)) => return closed(None, &e),
        Err(_) => return closed(None, "Timed out."),
    };
    let (mut sink, mut stream) = ws.split();
    if !emit(SocketEvent::Open) {
        let _ = sink.close().await;
        return closed(None, "The plugin fell behind.");
    }
    loop {
        tokio::select! {
            msg = stream.next() => {
                let taken = match msg {
                    Some(Ok(Message::Text(t))) => emit(SocketEvent::Text(t.to_string())),
                    Some(Ok(Message::Binary(b))) => emit(SocketEvent::Binary(b.to_vec())),
                    Some(Ok(Message::Close(frame))) => {
                        let code = frame.as_ref().map(|f| u16::from(f.code));
                        let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
                        return closed(code, &reason);
                    }
                    // Pings get their pongs from the library.
                    Some(Ok(_)) => true,
                    Some(Err(e)) => return closed(None, &short(&e)),
                    None => return closed(None, "Closed."),
                };
                if !taken {
                    let _ = sink.close().await;
                    return closed(None, "The plugin fell behind.");
                }
            }
            cmd = out.recv() => {
                let msg = match cmd {
                    Some(SocketOut::Text(t)) => Message::Text(t.into()),
                    Some(SocketOut::Binary(b)) => Message::Binary(b.into()),
                    // Closed by the plugin, dropped with its object, or the plugin stopped.
                    None => {
                        let _ = sink.close().await;
                        return closed(Some(1000), "");
                    }
                };
                if let Err(e) = sink.send(msg).await {
                    return closed(None, &short(&e));
                }
            }
        }
    }
}

async fn connect(req: &SocketRequest, local: bool) -> Result<Ws, String> {
    let url = Url::parse(&req.url).map_err(|_| "Can't read the address.".to_owned())?;
    let host = url.host_str().ok_or("The address has no host.")?.to_owned();
    let port = url.port_or_known_default().ok_or("The address has no port.")?;
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| format!("Can't find {host}."))?
        .collect();
    let addr = pick(&addrs, local).ok_or_else(|| format!("{host} points to a local address."))?;
    let tcp = TcpStream::connect(addr).await.map_err(|_| "Couldn't connect.".to_owned())?;
    let mut request = req.url.as_str().into_client_request().map_err(|e| short(&e))?;
    let headers = request.headers_mut();
    for (k, v) in &req.headers {
        let name = HeaderName::from_bytes(k.as_bytes()).map_err(|_| format!("Bad header {k}."))?;
        let value = HeaderValue::from_str(v).map_err(|_| format!("Bad header {k}."))?;
        headers.insert(name, value);
    }
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(concat!("cyex-viewer/", env!("CARGO_PKG_VERSION"), " (plugin)")),
    );
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let (ws, _) = tokio_tungstenite::client_async_tls_with_config(request, tcp, Some(config), None)
        .await
        .map_err(|e| short(&e))?;
    Ok(ws)
}

/// The first address that may be dialed.
pub(crate) fn pick(addrs: &[SocketAddr], local: bool) -> Option<SocketAddr> {
    addrs.iter().find(|a| local || public(a.ip())).copied()
}

fn short(e: &Error) -> String {
    match e {
        Error::Io(_) | Error::Tls(_) => "Couldn't connect.".into(),
        Error::Http(r) => format!("The server said {}.", r.status()),
        Error::Capacity(_) => "A message was over 1 MB.".into(),
        Error::ConnectionClosed | Error::AlreadyClosed => "Closed.".into(),
        other => other.to_string().chars().take(200).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_public_addresses_are_dialed() {
        let a = |s: &str| s.parse::<SocketAddr>().unwrap();
        let addrs = [a("127.0.0.1:80"), a("192.168.1.2:80"), a("93.184.216.34:80")];
        assert_eq!(pick(&addrs, false), Some(a("93.184.216.34:80")));
        assert_eq!(pick(&addrs[..2], false), None);
        assert_eq!(pick(&addrs[..2], true), Some(a("127.0.0.1:80")));
    }

    #[tokio::test]
    async fn relays_both_ways_and_closes() {
        use std::sync::{Arc, Mutex};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        // An echo server.
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            while let Some(Ok(m)) = ws.next().await {
                if m.is_text() || m.is_binary() {
                    ws.send(m).await.unwrap();
                }
            }
        });
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        let emit: Emit = Box::new(move |e| {
            log.lock().unwrap().push(e);
            true
        });
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let req = SocketRequest {
            url: format!("ws://127.0.0.1:{port}/"),
            headers: vec![("X-Test".into(), "1".into())],
        };
        let task = tokio::spawn(run_with(req, rx, emit, true));
        tx.send(SocketOut::Text("hi".into())).await.unwrap();
        tx.send(SocketOut::Binary(vec![1, 2])).await.unwrap();
        // Wait for both echoes, then close from our side.
        for _ in 0..100 {
            if seen.lock().unwrap().len() >= 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        drop(tx);
        task.await.unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0], SocketEvent::Open);
        assert_eq!(seen[1], SocketEvent::Text("hi".into()));
        assert_eq!(seen[2], SocketEvent::Binary(vec![1, 2]));
        assert!(matches!(seen.last(), Some(SocketEvent::Closed { code: Some(1000), .. })));
    }

    #[tokio::test]
    async fn local_addresses_are_refused() {
        let emit_log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = emit_log.clone();
        let emit: Emit = Box::new(move |e| {
            log.lock().unwrap().push(e);
            true
        });
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let req = SocketRequest {
            url: "ws://127.0.0.1:9/".into(),
            headers: Vec::new(),
        };
        run(req, rx, emit).await;
        assert!(matches!(&emit_log.lock().unwrap()[0], SocketEvent::Closed { code: None, reason } if reason.contains("local")));
    }
}
