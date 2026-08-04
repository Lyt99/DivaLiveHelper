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

use crate::logging::emit_log;
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

#[derive(Debug, Clone, Serialize)]
pub struct SongProcessOutcome {
    pub is_song_request: bool,
    pub query: String,
    pub matched: bool,
    pub song_id: Option<u32>,
    pub song_name: Option<String>,
    pub added: bool,
    pub requester: String,
    pub message: String,
}

/// 点歌失败通知（已识别为点歌意图，但匹配/入队失败时发送给前端）
#[derive(Debug, Clone, Serialize)]
pub struct SongRequestFailure {
    pub requester: String,
    pub query: String,
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
                        emit_log(&app, "弹幕连接已断开，5 秒后重连");
                        sleep(Duration::from_secs(5)).await;
                    }
                    Err(error) => {
                        emit_log(&app, format!("弹幕连接失败: {error}"));
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
    emit_log(
        &app,
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
            let outcome = process_song_request(app, &event).await;
            // 已识别为点歌意图但匹配/入队失败 → 推送失败通知
            // （LLM 判断不是点歌指令的情况 is_song_request=false，不算失败）
            if outcome.is_song_request && !outcome.added {
                let _ = app.emit(
                    "song-request-failed",
                    SongRequestFailure {
                        requester: outcome.requester.clone(),
                        query: outcome.query.clone(),
                        message: outcome.message.clone(),
                    },
                );
            }
            let _ = app.emit("danmaku", event);
        } else {
            emit_log(app, "收到弹幕消息，但解析字段失败");
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
        is_song_request: strip_command_prefix(&content, &config.song_command_prefix).is_some(),
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
        let raw = strip_command_prefix(&event.content, &config.song_command_prefix)
            .unwrap_or_else(|| event.content.trim().to_string());
        if let Some(song_name) = parse_prefix_request(&raw) {
            return enqueue_song(app, &song_name, &event.user_name, &config);
        }
        return SongProcessOutcome::miss(&event.user_name, &raw, "已识别点歌前缀，但缺少歌曲名");
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
            emit_log(app, format!("LLM 意图识别失败: {error}"));
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
    // 去除弹幕小表情代码（如 [喝彩]、[doge]），避免干扰歌名搜索
    let cleaned = strip_danmaku_emojis(song_name);
    let results = searcher.search(
        &cleaned,
        None,
        &config.default_search_difficulty,
        &config.difficulty_fallback,
        config.difficulty_tolerance,
    );
    enqueue_first_result(app, results.first(), requester, &cleaned)
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
    let cleaned = strip_danmaku_emojis(author);
    let results = searcher.search_by_author(
        &cleaned,
        None,
        &config.default_search_difficulty,
        &config.difficulty_fallback,
        config.difficulty_tolerance,
    );
    enqueue_first_result(app, results.first(), requester, &cleaned)
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
        emit_log(app, &message);
        return SongProcessOutcome::miss(requester, original_query, message);
    };
    let added = state.queue.add(
        result.pv_id,
        result.display_name.clone(),
        requester.to_string(),
        result.difficulty,
        // 优先使用搜索时实际匹配到的难度档位（可能经 fallback 得到，如 exextreme→extreme）；
        // 仅当歌曲完全无难度数据时才退回 config 默认档位。
        result.difficulty_tier.clone().unwrap_or_else(|| {
            state
                .config
                .read()
                .map(|cfg| cfg.default_search_difficulty.clone())
                .unwrap_or_else(|_| "extreme".to_string())
        }),
    );
    let message = if added {
        format!("已加入队列: {} (点歌人: {requester})", result.display_name)
    } else {
        format!("点歌失败（队列已满或重复）: {}", result.display_name)
    };
    emit_log(app, &message);
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

pub(crate) fn strip_command_prefix(content: &str, prefix: &str) -> Option<String> {
    let cleaned = strip_danmaku_emojis(content);
    let trimmed = cleaned.trim_start();
    trimmed
        .strip_prefix(prefix)
        .map(|request| request.trim().to_string())
}

/// 去除弹幕文本中的 B站小表情代码（`[xxx]` 格式，如 `[喝彩]`、`[doge]`、`[2233娘]`）。
///
/// 这些表情在弹幕 `info[1]` 文本字段中以方括号字面量出现，会干扰歌名搜索。
/// 使用逐字符扫描而非 regex crate，避免引入新依赖。
pub(crate) fn strip_danmaku_emojis(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            // 尝试找到匹配的 `]`；如果找到则跳过整段，否则保留 `[`
            let mut consumed = String::from('[');
            let mut found_close = false;
            while let Some(inner) = chars.next() {
                consumed.push(inner);
                if inner == ']' {
                    found_close = true;
                    break;
                }
            }
            if found_close && consumed.len() > 2 {
                // `[xxx]` —— 跳过（不写入 result）
            } else {
                // 无闭合 `]` 或 `[]` 空括号 —— 保留原文
                result.push_str(&consumed);
            }
        } else {
            result.push(ch);
        }
    }
    result
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

#[cfg(test)]
mod tests {
    use super::*;

    // ----------------- SongProcessOutcome constructors -----------------

    #[test]
    fn outcome_not_request_sets_all_none_fields() {
        let outcome = SongProcessOutcome::not_request("user1", "some reason");
        assert!(!outcome.is_song_request);
        assert!(!outcome.matched);
        assert!(!outcome.added);
        assert_eq!(outcome.song_id, None);
        assert_eq!(outcome.song_name, None);
        assert_eq!(outcome.query, "");
        assert_eq!(outcome.requester, "user1");
        assert_eq!(outcome.message, "some reason");
    }

    #[test]
    fn outcome_miss_marks_song_request_but_not_matched() {
        let outcome = SongProcessOutcome::miss("user2", "千本桜", "未找到匹配的歌曲");
        assert!(outcome.is_song_request);
        assert!(!outcome.matched);
        assert!(!outcome.added);
        assert_eq!(outcome.song_id, None);
        assert_eq!(outcome.song_name, None);
        assert_eq!(outcome.query, "千本桜");
        assert_eq!(outcome.requester, "user2");
    }

    // ----------------- parse_prefix_request -----------------

    #[test]
    fn parse_prefix_request_returns_trimmed_non_empty() {
        assert_eq!(parse_prefix_request("  hello  "), Some("hello".to_string()));
        assert_eq!(parse_prefix_request("千本桜"), Some("千本桜".to_string()));
    }

    #[test]
    fn parse_prefix_request_returns_none_for_empty_or_whitespace() {
        assert_eq!(parse_prefix_request(""), None);
        assert_eq!(parse_prefix_request("   "), None);
        assert_eq!(parse_prefix_request("\t\n"), None);
    }

    #[test]
    fn strip_command_prefix_ignores_danmaku_emojis_before_prefix() {
        assert_eq!(
            strip_command_prefix("[喝彩]点歌 Fire Flower", "点歌"),
            Some("Fire Flower".to_string())
        );
        assert_eq!(
            strip_command_prefix(" [doge][妙啊]点歌 千本桜", "点歌"),
            Some("千本桜".to_string())
        );
    }

    #[test]
    fn strip_command_prefix_removes_danmaku_emojis_inside_query() {
        assert_eq!(
            strip_command_prefix("点歌 [喝彩]Fire◎Flower[doge]", "点歌"),
            Some("Fire◎Flower".to_string())
        );
    }

    #[test]
    fn strip_command_prefix_returns_none_without_prefix_after_emoji_strip() {
        assert_eq!(strip_command_prefix("[喝彩]普通弹幕", "点歌"), None);
    }

    // ----------------- encode_packet / decode_packets -----------------

    #[test]
    fn encode_packet_produces_correct_header_layout() {
        let payload = b"hello";
        let encoded = encode_packet(7, payload);
        // 16 字节 header + 5 字节 payload = 21
        assert_eq!(encoded.len(), 16 + 5);
        // packet_len (u32 BE)
        assert_eq!(u32::from_be_bytes(encoded[0..4].try_into().unwrap()), 21);
        // header_len (u16 BE) = 16
        assert_eq!(u16::from_be_bytes(encoded[4..6].try_into().unwrap()), 16);
        // protocol (u16 BE) = 0 (PROTOCOL_JSON)
        assert_eq!(u16::from_be_bytes(encoded[6..8].try_into().unwrap()), 0);
        // operation (u32 BE) = 7
        assert_eq!(u32::from_be_bytes(encoded[8..12].try_into().unwrap()), 7);
        // sequence (u32 BE) = 1
        assert_eq!(u32::from_be_bytes(encoded[12..16].try_into().unwrap()), 1);
        // payload
        assert_eq!(&encoded[16..], b"hello");
    }

    #[test]
    fn encode_packet_with_empty_payload() {
        let encoded = encode_packet(2, b"");
        assert_eq!(encoded.len(), 16);
        assert_eq!(u32::from_be_bytes(encoded[0..4].try_into().unwrap()), 16);
    }

    #[test]
    fn decode_packets_round_trips_single_packet() {
        let original_payload = b"{\"cmd\":\"test\"}";
        let encoded = encode_packet(5, original_payload);
        let packets = decode_packets(&encoded).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].operation, 5);
        assert_eq!(packets[0].payload, original_payload);
    }

    #[test]
    fn decode_packets_round_trips_multiple_concatenated() {
        let p1 = encode_packet(1, b"one");
        let p2 = encode_packet(2, b"two");
        let mut combined = p1.clone();
        combined.extend_from_slice(&p2);
        let packets = decode_packets(&combined).unwrap();
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].operation, 1);
        assert_eq!(packets[0].payload, b"one");
        assert_eq!(packets[1].operation, 2);
        assert_eq!(packets[1].payload, b"two");
    }

    #[test]
    fn decode_packets_rejects_truncated_data() {
        // 不足 16 字节 header → 循环不执行，返回空 vec（非错误）
        let packets = decode_packets(b"short").unwrap();
        assert!(packets.is_empty());
    }

    #[test]
    fn decode_packets_rejects_zero_packet_len() {
        let mut bad = vec![0u8; 16];
        bad[0..4].copy_from_slice(&0u32.to_be_bytes()); // packet_len = 0
        assert!(decode_packets(&bad).is_err());
    }

    #[test]
    fn decode_packets_empty_input_returns_empty_vec() {
        let packets = decode_packets(b"").unwrap();
        assert!(packets.is_empty());
    }

    // ----------------- parse_auth_reply -----------------

    #[test]
    fn parse_auth_reply_success_when_code_zero() {
        let payload = br#"{"code":0,"message":"ok"}"#;
        assert!(parse_auth_reply(payload).is_ok());
    }

    #[test]
    fn parse_auth_reply_fails_with_message_when_nonzero() {
        let payload = "{\"code\":-101,\"message\":\"账号未登录\"}".as_bytes();
        let result = parse_auth_reply(payload);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("账号未登录"));
    }

    #[test]
    fn parse_auth_reply_falls_back_to_msg_field() {
        let payload = "{\"code\":1,\"msg\":\"fallback error\"}".as_bytes();
        let result = parse_auth_reply(payload);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fallback error"));
    }

    #[test]
    fn parse_auth_reply_falls_back_to_code_when_no_message() {
        let payload = br#"{"code":-352}"#;
        let result = parse_auth_reply(payload);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("-352"));
    }

    #[test]
    fn parse_auth_reply_falls_back_to_unknown_when_nothing() {
        let payload = br#"{"foo":"bar"}"#;
        let result = parse_auth_reply(payload);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未知错误"));
    }

    #[test]
    fn parse_auth_reply_rejects_invalid_json() {
        assert!(parse_auth_reply(b"not json").is_err());
    }

    // ----------------- extract_wbi_key -----------------

    #[test]
    fn extract_wbi_key_extracts_filename_stem() {
        let url = "https://example.com/path/img/wbi/img.png";
        assert_eq!(extract_wbi_key(Some(url)).unwrap(), "img");
    }

    #[test]
    fn extract_wbi_key_strips_query_and_fragment() {
        let url = "https://example.com/wbi/key.png?version=1#anchor";
        assert_eq!(extract_wbi_key(Some(url)).unwrap(), "key");
    }

    #[test]
    fn extract_wbi_key_returns_err_for_none() {
        assert!(extract_wbi_key(None).is_err());
    }

    #[test]
    fn extract_wbi_key_returns_err_for_url_without_extension() {
        // rsplit_once('.') 找不到 '.'，filename 为空 → Err
        let url = "https://example.com/noext";
        assert!(extract_wbi_key(Some(url)).is_err());
    }

    // ----------------- build_wbi_query -----------------

    #[test]
    fn build_wbi_query_encodes_key_value_pairs() {
        let mut params = BTreeMap::new();
        params.insert("foo".to_string(), "bar".to_string());
        params.insert("baz".to_string(), "qux".to_string());
        let query = build_wbi_query(&params);
        // BTreeMap 按字母排序
        assert_eq!(query, "baz=qux&foo=bar");
    }

    #[test]
    fn build_wbi_query_strips_filter_chars_from_values() {
        let mut params = BTreeMap::new();
        params.insert("key".to_string(), "val!'(ue)*".to_string());
        let query = build_wbi_query(&params);
        // !'()* 被剥离 → "value"
        assert_eq!(query, "key=value");
    }

    #[test]
    fn build_wbi_query_url_encodes_special_chars() {
        let mut params = BTreeMap::new();
        params.insert("k".to_string(), "a b".to_string());
        let query = build_wbi_query(&params);
        assert_eq!(query, "k=a%20b");
    }

    #[test]
    fn build_wbi_query_empty_params_returns_empty_string() {
        let params = BTreeMap::new();
        assert_eq!(build_wbi_query(&params), "");
    }

    // ----------------- DanmakuInfo::websocket_urls -----------------

    fn make_host(host: &str, wss_port: u64) -> DanmakuHost {
        DanmakuHost {
            host: host.to_string(),
            wss_port,
        }
    }

    #[test]
    fn websocket_urls_builds_wss_urls_from_valid_hosts() {
        let info = DanmakuInfo {
            token: "tok".to_string(),
            hosts: vec![make_host("a.com", 443), make_host("b.com", 712)],
        };
        let urls = info.websocket_urls();
        assert_eq!(urls, vec!["wss://a.com:443/sub", "wss://b.com:712/sub"]);
    }

    #[test]
    fn websocket_urls_filters_empty_host_and_zero_port() {
        let info = DanmakuInfo {
            token: "tok".to_string(),
            hosts: vec![
                make_host("", 443),
                make_host("b.com", 0),
                make_host("c.com", 712),
            ],
        };
        let urls = info.websocket_urls();
        assert_eq!(urls, vec!["wss://c.com:712/sub"]);
    }

    #[test]
    fn websocket_urls_falls_back_when_all_hosts_invalid() {
        let info = DanmakuInfo {
            token: "tok".to_string(),
            hosts: vec![make_host("", 0)],
        };
        let urls = info.websocket_urls();
        assert_eq!(urls, vec![format!("wss://{FALLBACK_WS_HOST}:443/sub")]);
    }

    #[test]
    fn websocket_urls_falls_back_when_empty() {
        let info = DanmakuInfo {
            token: "tok".to_string(),
            hosts: vec![],
        };
        let urls = info.websocket_urls();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].contains(FALLBACK_WS_HOST));
    }

    // ----------------- strip_danmaku_emojis -----------------

    #[test]
    fn strip_emojis_removes_single_cjk_emoji() {
        assert_eq!(strip_danmaku_emojis("[喝彩]点歌 起风了"), "点歌 起风了");
        assert_eq!(strip_danmaku_emojis("点歌[妙啊]千本桜"), "点歌千本桜");
    }

    #[test]
    fn strip_emojis_removes_ascii_emoji() {
        assert_eq!(strip_danmaku_emojis("[doge]点歌 MELTDOWN"), "点歌 MELTDOWN");
        assert_eq!(strip_danmaku_emojis("点歌 [awsl] test"), "点歌  test");
    }

    #[test]
    fn strip_emojis_removes_mixed_content_emoji() {
        assert_eq!(strip_danmaku_emojis("[2233娘]点歌 test"), "点歌 test");
        assert_eq!(strip_danmaku_emojis("[tv_白给]点歌"), "点歌");
    }

    #[test]
    fn strip_emojis_removes_multiple_emojis() {
        assert_eq!(strip_danmaku_emojis("[妙啊][赞]点歌 起风了"), "点歌 起风了");
        assert_eq!(strip_danmaku_emojis("[a][b]点歌[c]"), "点歌");
    }

    #[test]
    fn strip_emojis_removes_emoji_at_end() {
        assert_eq!(strip_danmaku_emojis("点歌 起风了[喝彩]"), "点歌 起风了");
    }

    #[test]
    fn strip_emojis_removes_emoji_in_middle() {
        assert_eq!(strip_danmaku_emojis("点歌[喝彩]起风了"), "点歌起风了");
    }

    #[test]
    fn strip_emojis_preserves_text_without_brackets() {
        assert_eq!(strip_danmaku_emojis("点歌 千本桜"), "点歌 千本桜");
        assert_eq!(strip_danmaku_emojis("Hello World"), "Hello World");
        assert_eq!(strip_danmaku_emojis(""), "");
    }

    #[test]
    fn strip_emojis_handles_pure_emoji_message() {
        assert_eq!(strip_danmaku_emojis("[喝彩]"), "");
        assert_eq!(strip_danmaku_emojis("[doge][妙啊]"), "");
    }

    #[test]
    fn strip_emojis_preserves_unclosed_bracket() {
        // 无闭合 `]` —— 保留原文
        assert_eq!(strip_danmaku_emojis("点歌 [unclosed"), "点歌 [unclosed");
    }

    #[test]
    fn strip_emojis_preserves_empty_brackets() {
        // `[]` 空括号 —— 保留（避免误删合法的空方括号）
        assert_eq!(strip_danmaku_emojis("点歌 []"), "点歌 []");
    }

    #[test]
    fn strip_emojis_preserves_adjacent_brackets() {
        // `[a][b]` —— 两个表情都被去除
        assert_eq!(strip_danmaku_emojis("[a][b]"), "");
        // 剩余的 `][` 是合法文本的一部分
        assert_eq!(strip_danmaku_emojis("x[a][b]y"), "xy");
    }
}
