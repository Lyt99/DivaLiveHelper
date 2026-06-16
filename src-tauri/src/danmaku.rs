use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::{BufMut, BytesMut};
use flate2::read::ZlibDecoder;
use futures_util::{SinkExt, Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::tungstenite::Message;

use crate::{llm_intent, AppState};

const ROOM_INIT_URL: &str = "https://api.live.bilibili.com/room/v1/Room/room_init";
const BUVID_SPI_URL: &str = "https://api.bilibili.com/x/frontend/finger/spi";
const WBI_NAV_URL: &str = "https://api.bilibili.com/x/web-interface/nav";
const DANMAKU_INFO_URL: &str = "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo";
const FALLBACK_WS_HOST: &str = "broadcastlv.chat.bilibili.com";
const REFERER_URL: &str = "https://live.bilibili.com/";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const HEADER_LEN: u16 = 16;
const PROTOCOL_JSON: u16 = 0;
const PROTOCOL_ZLIB: u16 = 2;
const PROTOCOL_BROTLI: u16 = 3;
const OP_HEARTBEAT: u32 = 2;
const OP_NOTIFICATION: u32 = 5;
const OP_AUTH: u32 = 7;
const OP_AUTH_REPLY: u32 = 8;

const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

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

#[derive(Debug, Clone)]
pub(crate) struct SongProcessOutcome {
    pub is_song_request: bool,
    pub query: String,
    pub matched: bool,
    pub song_id: Option<u32>,
    pub song_name: Option<String>,
    pub added: bool,
    pub requester: String,
    pub message: String,
}

#[derive(Debug, Clone)]
struct DanmakuInfo {
    token: String,
    hosts: Vec<DanmakuHost>,
}

impl DanmakuInfo {
    fn websocket_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = self
            .hosts
            .iter()
            .filter(|host| !host.host.trim().is_empty() && host.wss_port > 0)
            .map(|host| format!("wss://{}:{}/sub", host.host, host.wss_port))
            .collect();
        if urls.is_empty() {
            urls.push(format!("wss://{FALLBACK_WS_HOST}:443/sub"));
        }
        urls
    }
}

#[derive(Debug, Clone)]
struct DanmakuHost {
    host: String,
    wss_port: u64,
}

struct BilibiliHttpSession {
    client: Client,
    buvid3: Option<String>,
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
        self.task = Some(async_runtime::spawn(async move {
            loop {
                connected.store(false, Ordering::Relaxed);
                match run_client(app.clone(), room_id, connected.clone()).await {
                    Ok(()) => {
                        let _ = app.emit("log-event", "弹幕连接已断开，5 秒后重连");
                        sleep(Duration::from_secs(5)).await;
                    }
                    Err(error) => {
                        let _ = app.emit("log-event", format!("弹幕连接失败: {error}"));
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }));
    }
}

async fn build_http_session(sessdata: &str) -> Result<BilibiliHttpSession, String> {
    let mut headers = HeaderMap::new();
    headers.insert(REFERER, HeaderValue::from_static(REFERER_URL));

    let (cookie, buvid3) = build_bilibili_cookie(sessdata).await;
    if !cookie.is_empty() {
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&cookie)
                .map_err(|_| "SESSDATA 包含非法 HTTP 头字符".to_string())?,
        );
    }
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))?;
    Ok(BilibiliHttpSession { client, buvid3 })
}

async fn build_bilibili_cookie(sessdata: &str) -> (String, Option<String>) {
    let mut parts = Vec::new();
    if !sessdata.trim().is_empty() {
        parts.push(format!("SESSDATA={}", sessdata.trim()));
    }
    let mut auth_buvid3 = None;
    if let Ok((buvid3, buvid4)) = fetch_buvids().await {
        parts.push(format!("buvid3={buvid3}"));
        parts.push(format!("buvid4={buvid4}"));
        auth_buvid3 = Some(buvid3);
    }
    (parts.join("; "), auth_buvid3)
}

async fn fetch_buvids() -> Result<(String, String), String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| format!("创建 buvid 客户端失败: {error}"))?;
    let value = client
        .get(BUVID_SPI_URL)
        .header(REFERER, REFERER_URL)
        .send()
        .await
        .map_err(|error| format!("获取 buvid 失败: {error}"))?
        .json::<Value>()
        .await
        .map_err(|error| format!("解析 buvid 失败: {error}"))?;
    if value.get("code").and_then(Value::as_i64).unwrap_or(-1) != 0 {
        return Err("获取 buvid 失败".to_string());
    }
    let data = value
        .get("data")
        .ok_or_else(|| "buvid 响应缺少 data 字段".to_string())?;
    let buvid3 = data
        .get("b_3")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "buvid 响应缺少 b_3".to_string())?
        .to_string();
    let buvid4 = data
        .get("b_4")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "buvid 响应缺少 b_4".to_string())?
        .to_string();
    Ok((buvid3, buvid4))
}

async fn resolve_real_room_id(client: &Client, room_id: u64) -> Result<u64, String> {
    let response = client
        .get(ROOM_INIT_URL)
        .query(&[("id", room_id)])
        .send()
        .await
        .map_err(|error| format!("解析直播间 ID 失败: {error}"))?;
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| format!("解析直播间 ID 响应失败: {error}"))?;
    if value.get("code").and_then(Value::as_i64).unwrap_or(-1) != 0 {
        return Err(format!(
            "解析直播间 ID 失败: {}",
            value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("接口返回错误")
        ));
    }
    Ok(value
        .get("data")
        .and_then(|data| data.get("room_id"))
        .and_then(Value::as_u64)
        .unwrap_or(room_id))
}

async fn fetch_danmaku_info(client: &Client, room_id: u64) -> Result<DanmakuInfo, String> {
    let mixin_key = fetch_wbi_mixin_key(client).await?;
    let mut params = BTreeMap::from([
        ("id".to_string(), room_id.to_string()),
        ("type".to_string(), "0".to_string()),
        ("web_location".to_string(), "444.8".to_string()),
        ("wts".to_string(), now_timestamp().floor().to_string()),
    ]);
    let query = build_wbi_query(&params);
    let signature = format!("{:x}", md5::compute(format!("{query}{mixin_key}")));
    params.insert("w_rid".to_string(), signature);

    let query = build_wbi_query(&params);
    let response = client
        .get(format!("{DANMAKU_INFO_URL}?{query}"))
        .send()
        .await
        .map_err(|error| format!("获取弹幕服务器信息失败: {error}"))?;
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| format!("解析弹幕服务器信息失败: {error}"))?;
    if value.get("code").and_then(Value::as_i64).unwrap_or(-1) != 0 {
        return Err(format!(
            "获取弹幕服务器信息失败: {}",
            value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("接口返回错误")
        ));
    }

    let data = value
        .get("data")
        .ok_or_else(|| "弹幕服务器信息缺少 data 字段".to_string())?;
    let token = data
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| "弹幕服务器信息缺少 token".to_string())?
        .to_string();
    let hosts = data
        .get("host_list")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let wss_port = item.get("wss_port")?.as_u64()?;
                    Some(DanmakuHost {
                        host: item.get("host")?.as_str()?.to_string(),
                        wss_port,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(DanmakuInfo { token, hosts })
}

async fn fetch_wbi_mixin_key(client: &Client) -> Result<String, String> {
    let value = client
        .get(WBI_NAV_URL)
        .send()
        .await
        .map_err(|error| format!("获取 WBI 签名信息失败: {error}"))?
        .json::<Value>()
        .await
        .map_err(|error| format!("解析 WBI 签名信息失败: {error}"))?;
    let code = value.get("code").and_then(Value::as_i64).unwrap_or(-1);
    if code != 0 && code != -101 {
        return Err(format!(
            "获取 WBI 签名信息失败: {}",
            value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("接口返回错误")
        ));
    }
    let wbi_img = value
        .get("data")
        .and_then(|data| data.get("wbi_img"))
        .ok_or_else(|| "WBI 签名信息缺少 wbi_img".to_string())?;
    let img_key = extract_wbi_key(wbi_img.get("img_url").and_then(Value::as_str))?;
    let sub_key = extract_wbi_key(wbi_img.get("sub_url").and_then(Value::as_str))?;
    let raw_key = format!("{img_key}{sub_key}");
    let bytes = raw_key.as_bytes();
    if bytes.len() < MIXIN_KEY_ENC_TAB.len() {
        return Err("WBI 签名 key 长度非法".to_string());
    }
    let mixin_key: String = MIXIN_KEY_ENC_TAB
        .iter()
        .take(32)
        .map(|index| bytes[*index] as char)
        .collect();
    Ok(mixin_key)
}

fn extract_wbi_key(url: Option<&str>) -> Result<String, String> {
    let url = url.ok_or_else(|| "WBI 签名 URL 缺失".to_string())?;
    let filename = url
        .rsplit('/')
        .next()
        .map(|part| part.split(['?', '#']).next().unwrap_or(part))
        .and_then(|part| part.rsplit_once('.').map(|(stem, _)| stem))
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "WBI 签名 URL 格式非法".to_string())?;
    Ok(filename.to_string())
}

fn build_wbi_query(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(key, value)| {
            let filtered = value.replace(['!', '\'', '(', ')', '*'], "");
            format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(&filtered)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

async fn run_client(
    app: AppHandle,
    room_id: u64,
    connected: Arc<AtomicBool>,
) -> Result<(), String> {
    let sessdata = app
        .state::<AppState>()
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .sessdata
        .clone();
    let session = build_http_session(&sessdata).await?;
    let real_room_id = resolve_real_room_id(&session.client, room_id).await?;
    let danmaku_info = fetch_danmaku_info(&session.client, real_room_id).await?;
    let ws_urls = danmaku_info.websocket_urls();

    let mut websocket = None;
    let mut last_error = None;
    for ws_url in ws_urls {
        match tokio_tungstenite::connect_async(&ws_url).await {
            Ok((socket, _)) => {
                websocket = Some(socket);
                break;
            }
            Err(error) => {
                last_error = Some(format!("{ws_url}: {error}"));
            }
        }
    }
    let Some(mut websocket) = websocket else {
        return Err(format!(
            "连接弹幕服务器失败: {}",
            last_error.unwrap_or_else(|| "没有可用的弹幕服务器".to_string())
        ));
    };

    let mut auth = serde_json::json!({
        "uid": 0,
        "roomid": real_room_id,
        "protover": 3,
        "platform": "web",
        "type": 2,
        "key": danmaku_info.token,
    });
    if let Some(buvid3) = session.buvid3 {
        auth["buvid"] = Value::String(buvid3);
        auth["support_ack"] = Value::Bool(true);
    }
    websocket
        .send(Message::Binary(
            encode_packet(OP_AUTH, auth.to_string().as_bytes()).into(),
        ))
        .await
        .map_err(|error| format!("发送认证包失败: {error}"))?;
    wait_for_auth_reply(&mut websocket).await?;
    connected.store(true, Ordering::Relaxed);
    let _ = app.emit(
        "log-event",
        format!("弹幕认证通过，监听直播间 {real_room_id}"),
    );
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

async fn wait_for_auth_reply<S>(websocket: &mut S) -> Result<(), String>
where
    S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    timeout(Duration::from_secs(5), async {
        loop {
            let Some(message) = websocket.next().await else {
                return Err("弹幕认证前连接已断开".to_string());
            };
            let message = message.map_err(|error| format!("读取弹幕认证响应失败: {error}"))?;
            if let Message::Binary(data) = message {
                for packet in decode_packets(&data)? {
                    if packet.operation == OP_AUTH_REPLY {
                        return parse_auth_reply(&packet.payload);
                    }
                }
            }
        }
    })
    .await
    .map_err(|_| "弹幕认证超时".to_string())?
}

fn parse_auth_reply(payload: &[u8]) -> Result<(), String> {
    let value = serde_json::from_slice::<Value>(payload)
        .map_err(|error| format!("解析弹幕认证响应失败: {error}"))?;
    if value.get("code").and_then(Value::as_i64).unwrap_or(-1) == 0 {
        return Ok(());
    }
    let detail = value
        .get("message")
        .or_else(|| value.get("msg"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| value.get("code").map(Value::to_string))
        .unwrap_or_else(|| "未知错误".to_string());
    Err(format!("弹幕认证失败: {detail}"))
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
            let _ = process_song_request(app, &event).await;
            let _ = app.emit("danmaku", event);
        } else {
            let _ = app.emit("log-event", "收到弹幕消息，但解析字段失败");
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

pub(crate) async fn process_song_request(
    app: &AppHandle,
    event: &DanmakuEvent,
) -> SongProcessOutcome {
    let state = app.state::<AppState>();
    let Ok(config) = state.config.read().map(|config| config.clone()) else {
        return SongProcessOutcome::not_request(&event.user_name, "读取配置失败");
    };

    if event.is_song_request {
        let raw = event
            .content
            .trim_start()
            .trim_start_matches(&config.song_command_prefix)
            .trim();
        if let Some(song_name) = parse_prefix_request(raw) {
            return enqueue_song(app, &song_name, &event.user_name, &config);
        }
        return SongProcessOutcome::miss(&event.user_name, raw, "已识别点歌前缀，但缺少歌曲名");
    }

    if !config.llm_enabled {
        return SongProcessOutcome::not_request(
            &event.user_name,
            "未识别为点歌指令，且 LLM 未启用",
        );
    }

    match llm_intent::analyze(&config, &event.content).await {
        Ok(intent) if intent.is_song_request => {
            if let Some(author) = intent
                .author
                .as_deref()
                .filter(|_| intent.song_name.is_none())
            {
                enqueue_author(app, author, &event.user_name, &config)
            } else if let Some(song_name) = intent.song_name.as_deref() {
                enqueue_song(app, song_name, &event.user_name, &config)
            } else {
                SongProcessOutcome::miss(
                    &event.user_name,
                    &event.content,
                    "LLM 识别到点歌意图，但没有提取到歌名或作者",
                )
            }
        }
        Ok(_) => SongProcessOutcome::not_request(&event.user_name, "LLM 判断不是点歌指令"),
        Err(error) => {
            let _ = app.emit("log-event", format!("LLM 意图识别失败: {error}"));
            SongProcessOutcome::miss(
                &event.user_name,
                &event.content,
                format!("LLM 意图识别失败: {error}"),
            )
        }
    }
}

fn enqueue_song(
    app: &AppHandle,
    song_name: &str,
    requester: &str,
    config: &crate::config::Config,
) -> SongProcessOutcome {
    let state = app.state::<AppState>();
    let Ok(searcher) = state.searcher.read() else {
        return SongProcessOutcome::miss(requester, song_name, "读取搜索索引失败");
    };
    let results = searcher.search(
        song_name,
        None,
        &config.default_search_difficulty,
        &config.difficulty_fallback,
        config.difficulty_tolerance,
    );
    enqueue_first_result(app, results.first(), requester, song_name)
}

fn enqueue_author(
    app: &AppHandle,
    author: &str,
    requester: &str,
    config: &crate::config::Config,
) -> SongProcessOutcome {
    let state = app.state::<AppState>();
    let Ok(searcher) = state.searcher.read() else {
        return SongProcessOutcome::miss(requester, author, "读取搜索索引失败");
    };
    let results = searcher.search_by_author(
        author,
        None,
        &config.default_search_difficulty,
        &config.difficulty_fallback,
        config.difficulty_tolerance,
    );
    enqueue_first_result(app, results.first(), requester, author)
}

fn enqueue_first_result(
    app: &AppHandle,
    result: Option<&crate::song_search::SearchResult>,
    requester: &str,
    original_query: &str,
) -> SongProcessOutcome {
    let state = app.state::<AppState>();
    let Some(result) = result else {
        let message = format!("未找到匹配的歌曲: {original_query}");
        let _ = app.emit("log-event", message.clone());
        return SongProcessOutcome::miss(requester, original_query, message);
    };
    let added = state.queue.add(
        result.pv_id,
        result.display_name.clone(),
        requester.to_string(),
        result.difficulty,
        state
            .config
            .read()
            .map(|cfg| cfg.default_search_difficulty.clone())
            .unwrap_or_else(|_| "extreme".to_string()),
    );
    let message = if added {
        format!("已加入队列: {} (点歌人: {requester})", result.display_name)
    } else {
        format!("点歌失败（队列已满或重复）: {}", result.display_name)
    };
    let _ = app.emit("log-event", message);
    let _ = app.emit("queue-updated", state.queue.snapshot());
    SongProcessOutcome {
        is_song_request: true,
        query: original_query.to_string(),
        matched: true,
        song_id: Some(result.pv_id),
        song_name: Some(result.display_name.clone()),
        added,
        requester: requester.to_string(),
        message: if added {
            format!("已加入队列: {} (点歌人: {requester})", result.display_name)
        } else {
            format!("点歌失败（队列已满或重复）: {}", result.display_name)
        },
    }
}

impl SongProcessOutcome {
    fn not_request(requester: &str, message: impl Into<String>) -> Self {
        Self {
            is_song_request: false,
            query: String::new(),
            matched: false,
            song_id: None,
            song_name: None,
            added: false,
            requester: requester.to_string(),
            message: message.into(),
        }
    }

    fn miss(requester: &str, query: &str, message: impl Into<String>) -> Self {
        Self {
            is_song_request: true,
            query: query.to_string(),
            matched: false,
            song_id: None,
            song_name: None,
            added: false,
            requester: requester.to_string(),
            message: message.into(),
        }
    }
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
