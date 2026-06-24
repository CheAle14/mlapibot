use std::io::Cursor;
use std::io::Read;
use std::str::FromStr;

use anyhow::Context;
use mlapibot_common::action::PostAction;
use serde::Serialize;
use serde::de::DeserializeOwned;
use statuspage::incident::Incident;
use tiny_http::Header;
use tiny_http::Request;
use tiny_http::Response;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use mlapibot_common::config::ApiSettings;

const MAX_BODY_LENGTH: u64 = 1024 * 1024 * 10;

pub enum ApiEvent {
    WebhookRecv {
        incident: Option<Box<Incident>>,
    },

    GetRedditPost {
        link: String,
        reply: oneshot::Sender<GotRedditPost>,
    },

    RefreshStaffReply {
        subreddit_id: String,
        post_id: String,

        reply: oneshot::Sender<RefreshStaffResponse>,
    },

    AnalyzeInfo {
        subreddit_id: String,
        title: String,
        links: Vec<String>,
        body: Option<String>,

        reply: oneshot::Sender<GotAnalysis>,
    },

    PublishPost {
        id: i32,

        reply: oneshot::Sender<Option<String>>,
    },

    GetSubredditRemovalReasons {
        subreddit_id: String,

        reply: oneshot::Sender<SubredditRemovalReasonsResp>,
    },

    RefreshSubredditModerators {
        subreddit_id: String,

        reply: oneshot::Sender<()>,
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

            match url.as_str() {
                "/status" => {
                    let body = request.as_reader().take(MAX_BODY_LENGTH);

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
                    handle_request(&channel, request, |parsed: GetRedditReq, reply| {
                        ApiEvent::GetRedditPost {
                            link: parsed.link,
                            reply,
                        }
                    })
                }
                "/refresh-staff-reply" => {
                    handle_request(&channel, request, |parsed: RefreshStaffReq, reply| {
                        ApiEvent::RefreshStaffReply {
                            subreddit_id: parsed.subreddit_id,
                            post_id: parsed.post_id,
                            reply,
                        }
                    })
                }
                "/analyze" => handle_request(&channel, request, |parsed: AnalyzeReq, reply| {
                    ApiEvent::AnalyzeInfo {
                        subreddit_id: parsed.subreddit_id,
                        title: parsed.title,
                        links: parsed.links,
                        body: parsed.body,

                        reply,
                    }
                }),
                "/publish" => handle_request(&channel, request, |parsed: PublishPostReq, reply| {
                    ApiEvent::PublishPost {
                        id: parsed.id,
                        reply,
                    }
                }),
                "/removal-reasons" => {
                    handle_request(&channel, request, |parsed: SubredditIdReq, reply| {
                        ApiEvent::GetSubredditRemovalReasons {
                            subreddit_id: parsed.subreddit_id,
                            reply,
                        }
                    })
                }
                "/refresh-subreddit-moderators" => {
                    handle_request(&channel, request, |parsed: SubredditIdReq, reply| {
                        ApiEvent::RefreshSubredditModerators {
                            subreddit_id: parsed.subreddit_id,
                            reply,
                        }
                    })
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

fn handle_request<TReq, TConv, TResp>(
    channel: &mpsc::Sender<ApiEvent>,
    mut request: Request,
    converter: TConv,
) where
    TReq: DeserializeOwned,
    TConv: FnOnce(TReq, oneshot::Sender<TResp>) -> ApiEvent,
    TResp: Serialize,
{
    let body = request.as_reader().take(MAX_BODY_LENGTH);

    let parsed: TReq = match serde_json::from_reader(body) {
        Ok(value) => value,
        Err(err) => {
            println!("[status-webhook] {err:?}");
            let _ = request.respond(Response::empty(400));
            return;
        }
    };

    let (tx, rx) = oneshot::channel::<TResp>();
    channel.blocking_send(converter(parsed, tx)).unwrap();

    let _ = match rx.blocking_recv() {
        Ok(action) => request.respond(Response::from_json(&action).unwrap().with_status_code(200)),
        Err(_) => request.respond(Response::empty(500)),
    };
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct RefreshStaffReq {
    pub subreddit_id: String,
    pub post_id: String,
}

#[derive(serde::Serialize)]
pub struct RefreshStaffResponse {
    pub total_comments: u64,
    pub total_staff_comments: u64,
    pub new_staff_comments: u64,
}

#[derive(serde::Deserialize)]
struct AnalyzeReq {
    pub subreddit_id: String,
    pub title: String,
    #[serde(default)]
    pub links: Vec<String>,
    pub body: Option<String>,
}

#[derive(serde::Serialize)]
pub struct OcrImageData {
    pub name: String,
    pub text: String,
    pub triggers: Vec<usize>,
}

#[derive(serde::Serialize)]
pub struct GotAnalysis {
    pub action: PostAction,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ocr: Vec<OcrImageData>,
}

#[derive(serde::Deserialize)]
pub struct PublishPostReq {
    pub id: i32,
}

#[derive(serde::Serialize)]
pub struct PublishPostResponse {
    pub id: String,
}

#[derive(serde::Deserialize)]
pub struct SubredditIdReq {
    pub subreddit_id: String,
}

#[derive(serde::Serialize)]
pub struct RemovalReason {
    pub id: String,
    pub title: String,
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct SubredditRemovalReasonsResp {
    pub reasons: Vec<RemovalReason>,
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
