use std::io::Cursor;
use std::io::Read;
use std::str::FromStr;

use anyhow::Context;
use mlapibot_common::action::PostAction;
use statuspage::incident::Incident;
use tiny_http::Header;
use tiny_http::Response;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use mlapibot_common::config::ApiSettings;

pub enum ApiEvent {
    WebhookRecv {
        incident: Option<Box<Incident>>,
    },

    GetRedditPost {
        link: String,
        reply: oneshot::Sender<GotRedditPost>,
    },

    AnalyzeInfo {
        subreddit_id: String,
        title: String,
        link: Option<String>,
        body: Option<String>,

        reply: oneshot::Sender<PostAction>,
    },
}

pub fn start_web_connection(
    channel: mpsc::Sender<ApiEvent>,
    settings: &ApiSettings,
) -> anyhow::Result<()> {
    let socket = systemd_socket::SocketAddr::from_str(&settings.bind_address)
        .context("parse socket addr")?;

    println!("Using socket: {socket:#?}");

    let listener = socket.bind().context("create tcp listener")?;
    println!("Webhook listener is: {listener:?}");

    let access_token = settings.access_token.clone();

    let server = tiny_http::Server::from_listener(listener, None)
        .map_err(|e| anyhow::Error::from_boxed(e))
        .context("create server")?;

    std::thread::spawn(move || {
        loop {
            let mut request = match server.recv() {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("listen webhook err: {err}");
                    break;
                }
            };

            let url = match request
                .url()
                .trim_start_matches('/')
                .strip_prefix(&access_token)
            {
                None => {
                    let _ = request.respond(Response::empty(403));
                    continue;
                }
                Some(rest) => rest.to_owned(),
            };

            println!("[api] {} {}", request.method(), url);

            let body = request.as_reader().take(1024 * 1024 * 10);

            match url.as_str() {
                "/status" => {
                    let parsed: statuspage::webhook::StatusWebhook =
                        match serde_json::from_reader(body) {
                            Ok(value) => value,
                            Err(err) => {
                                println!("[status-webhook] {err:?}");
                                // *something* has happened, so trigger a refresh anyway

                                channel
                                    .blocking_send(ApiEvent::WebhookRecv { incident: None })
                                    .unwrap();

                                let _ = request.respond(Response::empty(500));
                                continue;
                            }
                        };

                    let event = match parsed.payload {
                        statuspage::webhook::WebhookPayload::Incident { incident } => {
                            ApiEvent::WebhookRecv {
                                incident: Some(Box::new(incident)),
                            }
                        }
                        _ => ApiEvent::WebhookRecv { incident: None },
                    };

                    channel.blocking_send(event).unwrap();

                    let _ = request.respond(Response::empty(204));
                }
                "/get-reddit" => {
                    let parsed: GetRedditReq = match serde_json::from_reader(body) {
                        Ok(value) => value,
                        Err(err) => {
                            println!("[status-webhook] {err:?}");
                            let _ = request.respond(Response::empty(400));
                            continue;
                        }
                    };

                    let (tx, rx) = oneshot::channel();
                    channel
                        .blocking_send(ApiEvent::GetRedditPost {
                            link: parsed.link,
                            reply: tx,
                        })
                        .unwrap();

                    let _ = match rx.blocking_recv() {
                        Ok(post) => request
                            .respond(Response::from_json(&post).unwrap().with_status_code(200)),
                        Err(_) => request.respond(Response::empty(500)),
                    };
                }
                "/analyze" => {
                    let parsed: AnalyzeReq = match serde_json::from_reader(body) {
                        Ok(value) => value,
                        Err(err) => {
                            println!("[status-webhook] {err:?}");
                            let _ = request.respond(Response::empty(400));
                            continue;
                        }
                    };

                    let (tx, rx) = oneshot::channel();
                    channel
                        .blocking_send(ApiEvent::AnalyzeInfo {
                            subreddit_id: parsed.subreddit_id,
                            title: parsed.title,
                            link: parsed.link,
                            body: parsed.body,

                            reply: tx,
                        })
                        .unwrap();

                    let _ = match rx.blocking_recv() {
                        Ok(action) => request
                            .respond(Response::from_json(&action).unwrap().with_status_code(200)),
                        Err(_) => request.respond(Response::empty(500)),
                    };
                }
                other => {
                    eprintln!("[api] unexpected request: {other:?}");
                    let _ = request.respond(Response::empty(404));
                }
            };
        }
    });

    Ok(())
}

#[derive(serde::Deserialize)]
struct GetRedditReq {
    link: String,
}

#[derive(serde::Serialize)]
pub struct GotRedditPost {
    pub subreddit_id: String,
    pub id: String,
    pub title: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(serde::Deserialize)]
struct AnalyzeReq {
    pub subreddit_id: String,
    pub title: String,
    pub link: Option<String>,
    pub body: Option<String>,
}

trait ResponseFromJson: Sized {
    fn from_json<T: serde::Serialize>(data: &T) -> serde_json::Result<Self>;
}

impl ResponseFromJson for tiny_http::Response<Cursor<Vec<u8>>> {
    fn from_json<T: serde::Serialize>(data: &T) -> serde_json::Result<Self> {
        let data = serde_json::to_vec(data)?;

        Ok(Response::from_data(data)
            .with_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap()))
    }
}
