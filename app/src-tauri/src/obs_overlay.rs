use std::sync::Arc;

use serde::Serialize;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::queue::SongQueue;

#[derive(Debug, Clone, Serialize)]
pub struct OBSOverlayStatus {
    pub running: bool,
    pub url: String,
    pub message: String,
}

pub struct OBSOverlayServer {
    host: String,
    port: u16,
    title: String,
    queue: Arc<SongQueue>,
    task: Option<JoinHandle<()>>,
}

impl OBSOverlayServer {
    pub fn new(host: String, port: u16, title: String, queue: Arc<SongQueue>) -> Self {
        Self {
            host,
            port,
            title,
            queue,
            task: None,
        }
    }

    pub fn update_config(&mut self, host: String, port: u16, title: String) {
        self.host = host;
        self.port = port;
        self.title = title;
    }

    pub fn status(&self) -> OBSOverlayStatus {
        OBSOverlayStatus {
            running: self.task.is_some(),
            url: self.url(),
            message: if self.task.is_some() {
                "OBS 覆盖层运行中".to_string()
            } else {
                "OBS 覆盖层未启动".to_string()
            },
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}/", self.host, self.port)
    }

    pub fn start(&mut self, app: &AppHandle) -> Result<(), String> {
        self.stop();
        if self.host != "127.0.0.1" && self.host != "localhost" {
            return Err("OBS 覆盖层仅允许绑定 127.0.0.1 或 localhost".to_string());
        }
        let host = self.host.clone();
        let port = self.port;
        let title = self.title.clone();
        let queue = self.queue.clone();
        let app = app.clone();
        let url = self.url();
        self.task = Some(async_runtime::spawn(async move {
            match TcpListener::bind((host.as_str(), port)).await {
                Ok(listener) => {
                    let _ = app.emit("log-event", format!("OBS 覆盖层已启动: {url}"));
                    loop {
                        match listener.accept().await {
                            Ok((stream, _)) => {
                                let queue = queue.clone();
                                let title = title.clone();
                                async_runtime::spawn(async move {
                                    let _ = handle_client(stream, queue, title).await;
                                });
                            }
                            Err(error) => {
                                let _ =
                                    app.emit("log-event", format!("OBS 覆盖层连接失败: {error}"));
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    let _ = app.emit("log-event", format!("OBS 覆盖层启动失败: {error}"));
                }
            }
        }));
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    queue: Arc<SongQueue>,
    title: String,
) -> Result<(), String> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .map_err(|error| format!("读取 HTTP 请求失败: {error}"))?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().copied().unwrap_or("");
    let path = parts
        .get(1)
        .copied()
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/");

    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .await
            .map_err(|error| format!("读取 HTTP 头失败: {error}"))?;
        if read == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }

    let (status, content_type, body, cors) = if method != "GET" {
        (
            405,
            "text/plain; charset=utf-8",
            b"Method Not Allowed".to_vec(),
            false,
        )
    } else if path == "/" || path == "/index.html" {
        (
            200,
            "text/html; charset=utf-8",
            render_overlay(&title).into_bytes(),
            false,
        )
    } else if path == "/api/queue" {
        let mut value = serde_json::to_value(queue.snapshot())
            .map_err(|error| format!("序列化队列失败: {error}"))?;
        if let Some(object) = value.as_object_mut() {
            object.insert("updated_at".to_string(), serde_json::json!(now_timestamp()));
        }
        let body =
            serde_json::to_vec(&value).map_err(|error| format!("序列化队列失败: {error}"))?;
        (200, "application/json; charset=utf-8", body, true)
    } else {
        (
            404,
            "text/plain; charset=utf-8",
            b"Not Found".to_vec(),
            false,
        )
    };

    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    };
    let cors_header = if cors {
        "Access-Control-Allow-Origin: *\r\n"
    } else {
        ""
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nCache-Control: no-store\r\n{cors_header}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut stream = reader.into_inner();
    stream
        .write_all(header.as_bytes())
        .await
        .map_err(|error| format!("写入 HTTP 头失败: {error}"))?;
    stream
        .write_all(&body)
        .await
        .map_err(|error| format!("写入 HTTP 响应失败: {error}"))?;
    Ok(())
}

fn render_overlay(title: &str) -> String {
    let title = escape_html(title);
    format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{title}</title><style>:root{{--bg:rgba(5,10,18,.72);--panel:rgba(10,20,34,.78);--cyan:#32f6ff;--pink:#ff4fd8;--gold:#ffe27a;--text:#f7fbff;--muted:rgba(247,251,255,.68);--line:rgba(50,246,255,.22);font-family:"Microsoft YaHei UI","Microsoft YaHei","Segoe UI",sans-serif}}*{{box-sizing:border-box}}body{{margin:0;min-height:100vh;color:var(--text);background:transparent;overflow:hidden}}.overlay{{width:min(680px,calc(100vw - 32px));margin:16px;padding:18px;border:1px solid var(--line);border-radius:24px;background:radial-gradient(circle at 12% 0%,rgba(255,79,216,.24),transparent 32%),radial-gradient(circle at 88% 12%,rgba(50,246,255,.26),transparent 34%),linear-gradient(135deg,var(--bg),var(--panel));box-shadow:0 24px 80px rgba(0,0,0,.42),inset 0 0 34px rgba(50,246,255,.08);backdrop-filter:blur(14px)}}header{{display:grid;grid-template-columns:1fr auto;gap:12px;align-items:end;margin-bottom:14px}}.eyebrow{{color:var(--cyan);font-size:12px;letter-spacing:.28em;text-transform:uppercase;text-shadow:0 0 14px rgba(50,246,255,.8)}}h1{{margin:2px 0 0;font-size:34px;line-height:1;letter-spacing:.04em;text-shadow:3px 3px 0 rgba(255,79,216,.72),0 0 24px rgba(50,246,255,.42)}}.counter{{min-width:92px;padding:10px 12px;border-radius:16px;background:rgba(0,0,0,.24);border:1px solid rgba(255,226,122,.28);text-align:center;color:var(--gold)}}.counter strong{{display:block;font-size:26px;line-height:1}}.counter span{{font-size:11px;color:var(--muted)}}.list{{display:grid;gap:10px;max-height:calc(100vh - 150px);overflow:hidden}}.song{{display:grid;grid-template-columns:44px 1fr auto;gap:12px;align-items:center;padding:12px 14px;border-radius:18px;background:linear-gradient(90deg,rgba(255,255,255,.11),rgba(255,255,255,.045));border:1px solid rgba(255,255,255,.08);animation:slide-in 360ms ease both}}.song:first-child{{background:linear-gradient(90deg,rgba(50,246,255,.24),rgba(255,79,216,.12));border-color:rgba(50,246,255,.34)}}.pos{{width:38px;height:38px;display:grid;place-items:center;border-radius:14px;color:#061019;background:linear-gradient(135deg,var(--cyan),var(--gold));font-weight:900;box-shadow:0 0 18px rgba(50,246,255,.35)}}.name{{font-size:20px;font-weight:800;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}.meta{{margin-top:3px;color:var(--muted);font-size:13px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}.difficulty{{padding:7px 10px;border-radius:999px;color:var(--pink);background:rgba(255,79,216,.12);border:1px solid rgba(255,79,216,.22);font-weight:800;white-space:nowrap}}.empty{{padding:34px 18px;border-radius:18px;border:1px dashed rgba(50,246,255,.28);color:var(--muted);text-align:center;background:rgba(0,0,0,.18)}}@keyframes slide-in{{from{{opacity:0;transform:translateX(-16px) scale(.98)}}to{{opacity:1;transform:translateX(0) scale(1)}}}}</style></head><body><main class="overlay"><header><div><div class="eyebrow">Project DIVA Live Helper</div><h1>{title}</h1></div><div class="counter"><strong id="count">0</strong><span>首等待</span></div></header><section class="list" id="list"><div class="empty"><strong>等待点歌中</strong><span>弹幕点歌会显示在这里</span></div></section></main><script>async function load(){{try{{const r=await fetch('/api/queue',{{cache:'no-store'}});const d=await r.json();document.getElementById('count').textContent=d.size||0;const list=document.getElementById('list');if(!d.songs||!d.songs.length){{list.innerHTML='<div class="empty"><strong>等待点歌中</strong><span>弹幕点歌会显示在这里</span></div>';return}}list.innerHTML=d.songs.map((s,i)=>`<article class="song" style="animation-delay:${{i*40}}ms"><div class="pos">${{s.position}}</div><div><div class="name">${{s.song_name}}</div><div class="meta">点歌人：${{s.requester}} · ID ${{s.song_id}}</div></div><div class="difficulty">${{s.difficulty?`${{s.difficulty}}★`:'READY'}}</div></article>`).join('')}}catch(e){{}}}}load();setInterval(load,2000)</script></body></html>"#
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn now_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}
