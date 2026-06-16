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
        let value = serde_json::to_value(queue.snapshot())
            .map_err(|error| format!("序列化队列失败: {error}"))?;
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
    r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__TITLE__</title>
  <style>
    :root {
      --panel: rgba(0, 0, 0, .55);
      --border: rgba(255, 255, 255, .08);
      --accent: #e0e0e0;
      --text: #ffffff;
      --muted: rgba(255, 255, 255, .50);
      --dim: rgba(255, 255, 255, .32);
      --playing: rgba(255, 255, 255, .07);
      font-family: "Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI", sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { min-height: 100vh; color: var(--text); background: transparent; overflow: hidden; }

    .overlay {
      width: min(520px, calc(100vw - 24px));
      margin: 12px;
      padding: 16px 18px;
      border-radius: 12px;
      background: var(--panel);
      border: 1px solid var(--border);
      backdrop-filter: blur(8px);
    }

    .header {
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      margin-bottom: 12px;
      padding-bottom: 10px;
      border-bottom: 1px solid var(--border);
    }
    .header h1 {
      font-size: 18px;
      font-weight: 600;
      letter-spacing: .02em;
    }
    .header .count {
      font-size: 13px;
      color: var(--muted);
    }

    .list { display: flex; flex-direction: column; gap: 6px; max-height: calc(100vh - 100px); overflow: hidden; }

    .song {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 8px 12px;
      border-radius: 8px;
      animation: fade-in 280ms ease both;
    }
    .song.playing {
      background: var(--playing);
    }
    .song .stars {
      font-size: 14px;
      color: var(--muted);
      white-space: nowrap;
      flex-shrink: 0;
      min-width: 36px;
      text-align: center;
    }
    .song.playing .stars {
      color: var(--accent);
    }
    .song .name {
      font-size: 18px;
      font-weight: 600;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .empty {
      padding: 24px 16px;
      border-radius: 8px;
      border: 1px dashed var(--border);
      text-align: center;
      color: var(--muted);
      font-size: 14px;
    }

    @keyframes fade-in { from { opacity: 0 } to { opacity: 1 } }
  </style>
</head>
<body>
  <main class="overlay">
    <div class="header"><h1>__TITLE__</h1><span class="count"><strong id="count">0</strong> 首等待</span></div>
    <section class="list" id="list"><div class="empty">等待点歌…</div></section>
  </main>
  <script>
    let lastSignature = '';
    const escapeHtml = (value) => String(value ?? '').replace(/[&<>'"]/g, (char) => ({'&':'&amp;','<':'&lt;','>':'&gt;',"'":'&#39;','"':'&quot;'}[char]));
    async function load(){
      try{
        const response = await fetch('/api/queue', { cache: 'no-store' });
        const data = await response.json();
        document.getElementById('count').textContent = data.size || 0;
        const songs = data.songs || [];
        const signature = JSON.stringify(songs.map((song) => [song.position, song.song_id, song.song_name, song.requester, song.difficulty]));
        if (signature === lastSignature) return;
        lastSignature = signature;
        const list = document.getElementById('list');
        if (!songs.length) {
          list.innerHTML = '<div class="empty">等待点歌…</div>';
          return;
        }
        list.innerHTML = songs.map((song, index) => `<div class="song${index === 0 ? ' playing' : ''}" style="animation-delay:${index * 30}ms"><span class="stars">${song.difficulty != null ? escapeHtml(song.difficulty) + '★' : ''}</span><span class="name">${escapeHtml(song.song_name)}</span></div>`).join('');
      } catch (_error) {}
    }
    load();
    setInterval(load, 2000);
  </script>
</body>
</html>"#
        .replace("__TITLE__", &title)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
