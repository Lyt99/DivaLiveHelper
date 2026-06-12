use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::config::Config;
use crate::danmaku::{self, DanmakuEvent, DanmakuStatus, SongProcessOutcome};
use crate::db_tool::{self, RebuildReport};
use crate::hotkey::HotkeyStatus;
use crate::obs_overlay::OBSOverlayStatus;
use crate::queue::SongRequest;
use crate::song_db::{SongDatabase, SongInfo};
use crate::song_search::{SearchResult, SongSearcher};
use crate::{play_next_song, AppState};

#[derive(Debug, Clone, Serialize)]
pub struct DebugSongRequestResult {
    pub is_song_request: bool,
    pub query: String,
    pub matched: bool,
    pub song_id: Option<u32>,
    pub song_name: Option<String>,
    pub added: bool,
    pub requester: Option<String>,
    pub message: String,
}

impl From<SongProcessOutcome> for DebugSongRequestResult {
    fn from(outcome: SongProcessOutcome) -> Self {
        Self {
            is_song_request: outcome.is_song_request,
            query: outcome.query,
            matched: outcome.matched,
            song_id: outcome.song_id,
            song_name: outcome.song_name,
            added: outcome.added,
            requester: Some(outcome.requester),
            message: outcome.message,
        }
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    state
        .config
        .read()
        .map(|config| config.clone())
        .map_err(|_| "读取配置锁失败".to_string())
}

#[tauri::command]
pub fn save_config(config: Config, state: State<'_, AppState>) -> Result<(), String> {
    config.validate()?;
    config.save(&state.config_path)?;
    let mut current = state
        .config
        .write()
        .map_err(|_| "写入配置锁失败".to_string())?;
    *current = config;
    Ok(())
}

#[tauri::command]
pub fn validate_config(config: Config) -> Result<String, String> {
    config.validate()?;
    Ok("配置有效".to_string())
}

#[tauri::command]
pub fn get_all_songs(state: State<'_, AppState>) -> Result<Vec<SongInfo>, String> {
    Ok(state
        .database
        .read()
        .map_err(|_| "读取歌曲库锁失败".to_string())?
        .to_song_infos())
}

#[tauri::command]
pub fn reload_database(state: State<'_, AppState>) -> Result<usize, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .clone();
    let fallback = state
        .data_dir
        .read()
        .map_err(|_| "读取数据目录锁失败".to_string())?
        .clone();
    let data_dir = resolve_data_dir(&config.data_dir, &fallback);
    let database = SongDatabase::load_with_chinese_names(&data_dir)?;
    let searcher = SongSearcher::from_database(&database, &data_dir);
    let count = database.songs.len();
    *state
        .database
        .write()
        .map_err(|_| "写入歌曲库锁失败".to_string())? = database;
    *state
        .searcher
        .write()
        .map_err(|_| "写入搜索索引锁失败".to_string())? = searcher;
    *state
        .data_dir
        .write()
        .map_err(|_| "写入数据目录锁失败".to_string())? = data_dir;
    Ok(count)
}

#[tauri::command]
pub fn rebuild_database(state: State<'_, AppState>) -> Result<RebuildReport, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .clone();
    let fallback = state
        .data_dir
        .read()
        .map_err(|_| "读取数据目录锁失败".to_string())?
        .clone();
    let data_dir = resolve_data_dir(&config.data_dir, &fallback);
    let mods_dir = if config.mods_dir.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(&config.mods_dir))
    };
    let report = db_tool::rebuild_database(&data_dir, mods_dir.as_deref())?;

    let database = SongDatabase::load_with_chinese_names(&data_dir)?;
    let searcher = SongSearcher::from_database(&database, &data_dir);
    *state
        .database
        .write()
        .map_err(|_| "写入歌曲库锁失败".to_string())? = database;
    *state
        .searcher
        .write()
        .map_err(|_| "写入搜索索引锁失败".to_string())? = searcher;
    *state
        .data_dir
        .write()
        .map_err(|_| "写入数据目录锁失败".to_string())? = data_dir;
    Ok(report)
}

#[tauri::command]
pub fn search_songs(
    query: String,
    difficulty: Option<f32>,
    difficulty_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .clone();
    let key = difficulty_key.unwrap_or(config.default_search_difficulty);
    Ok(state
        .searcher
        .read()
        .map_err(|_| "读取搜索索引锁失败".to_string())?
        .search(&query, difficulty, &key, config.difficulty_tolerance))
}

#[tauri::command]
pub fn search_songs_by_author(
    author: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .clone();
    Ok(state
        .searcher
        .read()
        .map_err(|_| "读取搜索索引锁失败".to_string())?
        .search_by_author(
            &author,
            None,
            &config.default_search_difficulty,
            config.difficulty_tolerance,
        ))
}

#[tauri::command]
pub fn debug_song_request(
    text: String,
    state: State<'_, AppState>,
) -> Result<DebugSongRequestResult, String> {
    resolve_debug_song_request(&text, &state)
}

#[tauri::command]
pub async fn debug_enqueue_song(
    text: String,
    app: AppHandle,
) -> Result<DebugSongRequestResult, String> {
    let state = app.state::<AppState>();
    let config = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .clone();
    let trimmed = text.trim_start();
    let is_song_request = trimmed.starts_with(&config.song_command_prefix);
    let event = DanmakuEvent {
        user_name: "调试".to_string(),
        content: text,
        is_song_request,
        timestamp: current_timestamp(),
    };
    Ok(danmaku::process_song_request(&app, &event).await.into())
}

fn resolve_debug_song_request(
    text: &str,
    state: &AppState,
) -> Result<DebugSongRequestResult, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .clone();
    let trimmed = text.trim_start();
    if !trimmed.starts_with(&config.song_command_prefix) {
        return Ok(DebugSongRequestResult {
            is_song_request: false,
            query: String::new(),
            matched: false,
            song_id: None,
            song_name: None,
            added: false,
            requester: None,
            message: format!(
                "未识别为点歌指令：当前前缀是 {}",
                config.song_command_prefix
            ),
        });
    }

    let query = trimmed
        .trim_start_matches(&config.song_command_prefix)
        .trim()
        .to_string();
    if query.is_empty() {
        return Ok(DebugSongRequestResult {
            is_song_request: true,
            query,
            matched: false,
            song_id: None,
            song_name: None,
            added: false,
            requester: None,
            message: "已识别点歌前缀，但缺少歌曲名".to_string(),
        });
    }

    let results = state
        .searcher
        .read()
        .map_err(|_| "读取搜索索引锁失败".to_string())?
        .search(
            &query,
            None,
            &config.default_search_difficulty,
            config.difficulty_tolerance,
        );
    let Some(result) = results.first() else {
        return Ok(DebugSongRequestResult {
            is_song_request: true,
            query: query.clone(),
            matched: false,
            song_id: None,
            song_name: None,
            added: false,
            requester: None,
            message: format!("点歌指令有效，但没有找到匹配歌曲：{query}"),
        });
    };

    Ok(DebugSongRequestResult {
        is_song_request: true,
        query,
        matched: true,
        song_id: Some(result.pv_id),
        song_name: Some(result.display_name.clone()),
        added: false,
        requester: None,
        message: format!("可点歌：{} (#{})", result.display_name, result.pv_id),
    })
}

#[tauri::command]
pub fn get_queue(state: State<'_, AppState>) -> Result<Vec<SongRequest>, String> {
    Ok(state.queue.list())
}

#[tauri::command]
pub fn get_queue_history(state: State<'_, AppState>) -> Result<Vec<SongRequest>, String> {
    Ok(state.queue.history())
}

#[tauri::command]
pub fn remove_from_queue(song_id: u32, state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.queue.remove(song_id))
}

#[tauri::command]
pub fn clear_queue(state: State<'_, AppState>) -> Result<(), String> {
    state.queue.clear();
    Ok(())
}

#[tauri::command]
pub fn next_song(app: AppHandle) -> Result<Option<SongRequest>, String> {
    play_next_song(&app)
}

#[tauri::command]
pub fn change_song(song_id: u32, state: State<'_, AppState>) -> Result<String, String> {
    if !state
        .searcher
        .read()
        .map_err(|_| "读取搜索索引锁失败".to_string())?
        .check_id(song_id)
    {
        return Err("歌曲ID不存在".to_string());
    }
    state
        .selector
        .lock()
        .map_err(|_| "切歌器锁失败".to_string())?
        .change_song(song_id)
}

#[tauri::command]
pub fn get_game_connection_status(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state
        .selector
        .lock()
        .map_err(|_| "切歌器锁失败".to_string())?
        .is_connected())
}

#[tauri::command]
pub fn reconnect_game(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state
        .selector
        .lock()
        .map_err(|_| "切歌器锁失败".to_string())?
        .reconnect())
}

#[tauri::command]
pub fn start_danmaku(
    room_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let effective_room_id = if room_id == 0 {
        state
            .config
            .read()
            .map_err(|_| "读取配置锁失败".to_string())?
            .room_id
    } else {
        room_id
    };
    if effective_room_id == 0 {
        return Err("未设置直播间ID".to_string());
    }
    state
        .danmaku
        .lock()
        .map_err(|_| "弹幕管理器锁失败".to_string())?
        .start(app, effective_room_id);
    Ok(())
}

#[tauri::command]
pub fn stop_danmaku(state: State<'_, AppState>) -> Result<(), String> {
    state
        .danmaku
        .lock()
        .map_err(|_| "弹幕管理器锁失败".to_string())?
        .stop();
    Ok(())
}

#[tauri::command]
pub fn get_danmaku_status(state: State<'_, AppState>) -> Result<DanmakuStatus, String> {
    Ok(state
        .danmaku
        .lock()
        .map_err(|_| "弹幕管理器锁失败".to_string())?
        .status())
}

#[tauri::command]
pub fn register_hotkey(app: AppHandle, state: State<'_, AppState>) -> Result<HotkeyStatus, String> {
    let hotkey = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .hotkey
        .clone();
    let mut manager = state
        .hotkey
        .lock()
        .map_err(|_| "快捷键管理器锁失败".to_string())?;
    manager.update_and_register(&app, hotkey)?;
    Ok(manager.status())
}

#[tauri::command]
pub fn unregister_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<HotkeyStatus, String> {
    let mut manager = state
        .hotkey
        .lock()
        .map_err(|_| "快捷键管理器锁失败".to_string())?;
    manager.unregister(&app)?;
    Ok(manager.status())
}

#[tauri::command]
pub fn get_hotkey_status(state: State<'_, AppState>) -> Result<HotkeyStatus, String> {
    Ok(state
        .hotkey
        .lock()
        .map_err(|_| "快捷键管理器锁失败".to_string())?
        .status())
}

#[tauri::command]
pub fn start_obs_overlay(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OBSOverlayStatus, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "读取配置锁失败".to_string())?
        .clone();
    let mut server = state
        .obs_overlay
        .lock()
        .map_err(|_| "OBS 覆盖层锁失败".to_string())?;
    server.update_config(
        config.obs_overlay_host,
        config.obs_overlay_port,
        config.obs_overlay_title,
    );
    server.start(&app)?;
    Ok(server.status())
}

#[tauri::command]
pub fn stop_obs_overlay(state: State<'_, AppState>) -> Result<OBSOverlayStatus, String> {
    let mut server = state
        .obs_overlay
        .lock()
        .map_err(|_| "OBS 覆盖层锁失败".to_string())?;
    server.stop();
    Ok(server.status())
}

#[tauri::command]
pub fn get_obs_overlay_status(state: State<'_, AppState>) -> Result<OBSOverlayStatus, String> {
    Ok(state
        .obs_overlay
        .lock()
        .map_err(|_| "OBS 覆盖层锁失败".to_string())?
        .status())
}

fn resolve_data_dir(configured: &str, fallback: &Path) -> PathBuf {
    let configured_path = PathBuf::from(configured);
    if configured_path.is_absolute() || configured_path.exists() {
        configured_path
    } else {
        fallback.join(configured)
    }
}

fn current_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}
