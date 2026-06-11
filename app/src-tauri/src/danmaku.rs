use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::{BufMut, BytesMut};
use flate2::read::ZlibDecoder;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;

use crate::{llm_intent, AppState};

const WS_URL: &str = "wss://broadcastlv.chat.bilibili.com/sub";
const HEADER_LEN: u16 = 16;
const PROTOCOL_JSON: u16 = 0;
const PROTOCOL_ZLIB: u16 = 2;
const PROTOCOL_BROTLI: u16 = 3;
const OP_HEARTBEAT: u32 = 2;
const OP_NOTIFICATION: u32 = 5;
const OP_AUTH: u32 = 7;
const OP_AUTH_REPLY: u32 = 8;

#[derive(Debug, Clone, Serialize)]
pub struct DanmakuEvent {
    pub user_name: String,
    pub content: String,
    pub is_song_request: bool,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DanmakuStatus {
    pub connected: bool,
    pub room_id: u64,
}

#[derive(Default)]
pub struct DanmakuManager {
    connected: Arc<AtomicBool>,
    room_id: u64,
    task: Option<JoinHandle<()>>,
}

impl DanmakuManager {
    pub fn status(&self) -> DanmakuStatus {
        DanmakuStatus {
            connected: self.connected.load(Ordering::Relaxed),
            room_id: self.room_id,
        }
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.connected.store(false, Ordering::Relaxed);
    }

    pub fn start(&mut self, app: AppHandle, room_id: u64) {
        self.stop();
        self.room_id = room_id;
        let connected = self.connected.clone();
        self.task = Some(tokio::spawn(async move {
            loop {
                connected.store(false, Ordering::Relaxed);
                match run_client(app.clone(), room_id, connected.clone()).await {
                    Ok(()) => break,
                    Err(error) => {
                        let _ = app.emit("log-event", format!("弹幕连接失败: {error}"));
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }));
    }
}

async fn run_client(
    app: AppHandle,
    room_id: u64,
    connected: Arc<AtomicBool>,
) -> Result<(), String> {
    let (mut websocket, _) = tokio_tungstenite::connect_async(WS_URL)
        .await
        .map_err(|error| format!("连接弹幕服务器失败: {error}"))?;
    let auth = serde_json::json!({
        "uid": 0,
        "roomid": room_id,
        "protover": 3,
        "platform": "web",
        "type": 2,
        "key": "",
    });
    websocket
        .send(Message::Binary(
            encode_packet(OP_AUTH, auth.to_string().as_bytes()).into(),
        ))
        .await
        .map_err(|error| format!("发送认证包失败: {error}"))?;
    connected.store(true, Ordering::Relaxed);
    let _ = app.emit("connection-status", "弹幕已连接");

    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                websocket.send(Message::Binary(encode_packet(OP_HEARTBEAT, b"[object Object]").into())).await.map_err(|error| format!("发送心跳失败: {error}"))?;
            }
            message = websocket.next() => {
                let Some(message) = message else { break; };
                let message = message.map_err(|error| format!("读取弹幕消息失败: {error}"))?;
                if let Message::Binary(data) = message {
                    for packet in decode_packets(&data)? {
                        handle_packet(&app, packet.operation, &packet.payload).await;
                    }
                }
            }
        }
    }
    connected.store(false, Ordering::Relaxed);
    Ok(())
}

async fn handle_packet(app: &AppHandle, operation: u32, payload: &[u8]) {
    if operation != OP_NOTIFICATION && operation != OP_AUTH_REPLY {
        return;
    }
    let Ok(value) = serde_json::from_slice::<Value>(payload) else {
        return;
    };
    if value
        .get("cmd")
        .and_then(Value::as_str)
        .is_some_and(|cmd| cmd.starts_with("DANMU_MSG"))
    {
        if let Some(event) = parse_danmaku_event(app, value) {
            process_song_request(app, &event).await;
            let _ = app.emit("danmaku", event);
        }
    }
}

fn parse_danmaku_event(app: &AppHandle, value: Value) -> Option<DanmakuEvent> {
    let state = app.state::<AppState>();
    let config = state.config.read().ok()?.clone();
    let info = value.get("info")?.as_array()?;
    let content = info.get(1)?.as_str()?.to_string();
    let user_name = info.get(2)?.as_array()?.get(1)?.as_str()?.to_string();
    Some(DanmakuEvent {
        is_song_request: content
            .trim_start()
            .starts_with(&config.song_command_prefix),
        user_name,
        content,
        timestamp: now_timestamp(),
    })
}

async fn process_song_request(app: &AppHandle, event: &DanmakuEvent) {
    let state = app.state::<AppState>();
    let Ok(config) = state.config.read().map(|config| config.clone()) else {
        return;
    };

    if event.is_song_request {
        let raw = event
            .content
            .trim_start()
            .trim_start_matches(&config.song_command_prefix)
            .trim();
        if let Some(song_name) = parse_prefix_request(raw) {
            enqueue_song(app, &song_name, &event.user_name, &config);
        }
        return;
    }

    if !config.llm_enabled || config.llm_api_key.trim().is_empty() {
        return;
    }

    match llm_intent::analyze(&config, &event.content).await {
        Ok(intent) if intent.is_song_request => {
            if let Some(author) = intent
                .author
                .as_deref()
                .filter(|_| intent.song_name.is_none())
            {
                enqueue_author(app, author, &event.user_name, &config);
            } else if let Some(song_name) = intent.song_name.as_deref() {
                enqueue_song(app, song_name, &event.user_name, &config);
            }
        }
        Ok(_) => {}
        Err(error) => {
            let _ = app.emit("log-event", format!("LLM 意图识别失败: {error}"));
        }
    }
}

fn enqueue_song(app: &AppHandle, song_name: &str, requester: &str, config: &crate::config::Config) {
    let state = app.state::<AppState>();
    let Ok(searcher) = state.searcher.read() else {
        return;
    };
    let results = searcher.search(
        song_name,
        None,
        &config.default_search_difficulty,
        config.difficulty_tolerance,
    );
    enqueue_first_result(app, results.first(), requester, song_name);
}

fn enqueue_author(app: &AppHandle, author: &str, requester: &str, config: &crate::config::Config) {
    let state = app.state::<AppState>();
    let Ok(searcher) = state.searcher.read() else {
        return;
    };
    let results = searcher.search_by_author(
        author,
        None,
        &config.default_search_difficulty,
        config.difficulty_tolerance,
    );
    enqueue_first_result(app, results.first(), requester, author);
}

fn enqueue_first_result(
    app: &AppHandle,
    result: Option<&crate::song_search::SearchResult>,
    requester: &str,
    original_query: &str,
) {
    let state = app.state::<AppState>();
    let Some(result) = result else {
        let _ = app.emit("log-event", format!("未找到匹配的歌曲: {original_query}"));
        return;
    };
    let added = state.queue.add(
        result.pv_id,
        result.display_name.clone(),
        requester.to_string(),
        None,
    );
    let message = if added {
        format!("已加入队列: {} (点歌人: {requester})", result.display_name)
    } else {
        format!("点歌失败（队列已满或重复）: {}", result.display_name)
    };
    let _ = app.emit("log-event", message);
    let _ = app.emit("queue-updated", state.queue.snapshot());
}

fn parse_prefix_request(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    Some(raw.to_string())
}

#[derive(Debug)]
struct Packet {
    operation: u32,
    payload: Vec<u8>,
}

fn encode_packet(operation: u32, payload: &[u8]) -> Vec<u8> {
    let packet_len = HEADER_LEN as usize + payload.len();
    let mut buffer = BytesMut::with_capacity(packet_len);
    buffer.put_u32(packet_len as u32);
    buffer.put_u16(HEADER_LEN);
    buffer.put_u16(PROTOCOL_JSON);
    buffer.put_u32(operation);
    buffer.put_u32(1);
    buffer.extend_from_slice(payload);
    buffer.to_vec()
}

fn decode_packets(data: &[u8]) -> Result<Vec<Packet>, String> {
    let mut packets = Vec::new();
    let mut offset = 0usize;
    while offset + HEADER_LEN as usize <= data.len() {
        let packet_len = u32::from_be_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|_| "弹幕包长度解析失败")?,
        ) as usize;
        let header_len = u16::from_be_bytes(
            data[offset + 4..offset + 6]
                .try_into()
                .map_err(|_| "弹幕包头长度解析失败")?,
        ) as usize;
        let protocol = u16::from_be_bytes(
            data[offset + 6..offset + 8]
                .try_into()
                .map_err(|_| "弹幕协议版本解析失败")?,
        );
        let operation = u32::from_be_bytes(
            data[offset + 8..offset + 12]
                .try_into()
                .map_err(|_| "弹幕操作码解析失败")?,
        );
        if packet_len == 0 || offset + packet_len > data.len() || header_len > packet_len {
            return Err("弹幕包长度非法".to_string());
        }
        let payload = &data[offset + header_len..offset + packet_len];
        match protocol {
            PROTOCOL_ZLIB => {
                let mut decoder = ZlibDecoder::new(payload);
                let mut decompressed = Vec::new();
                std::io::Read::read_to_end(&mut decoder, &mut decompressed)
                    .map_err(|error| format!("zlib 解压弹幕失败: {error}"))?;
                packets.extend(decode_packets(&decompressed)?);
            }
            PROTOCOL_BROTLI => {
                let mut decompressed = Vec::new();
                let mut reader = brotli::Decompressor::new(payload, 4096);
                std::io::Read::read_to_end(&mut reader, &mut decompressed)
                    .map_err(|error| format!("brotli 解压弹幕失败: {error}"))?;
                packets.extend(decode_packets(&decompressed)?);
            }
            _ => packets.push(Packet {
                operation,
                payload: payload.to_vec(),
            }),
        }
        offset += packet_len;
    }
    Ok(packets)
}

fn now_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}
