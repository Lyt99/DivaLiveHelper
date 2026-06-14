mod commands;
mod config;
mod danmaku;
mod db_tool;
mod hotkey;
mod llm_intent;
mod obs_overlay;
mod queue;
mod song_db;
mod song_search;
mod song_select;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use config::Config;
use danmaku::DanmakuManager;
use hotkey::HotkeyManager;
use obs_overlay::OBSOverlayServer;
use queue::SongQueue;
use queue::SongRequest;
use song_db::SongDatabase;
use song_search::SongSearcher;
use song_select::SongSelector;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

pub struct AppState {
    pub config_path: PathBuf,
    pub data_dir: Arc<RwLock<PathBuf>>,
    pub config: Arc<RwLock<Config>>,
    pub queue: Arc<SongQueue>,
    pub database: Arc<RwLock<SongDatabase>>,
    pub searcher: Arc<RwLock<SongSearcher>>,
    pub selector: Arc<Mutex<SongSelector>>,
    pub danmaku: Arc<Mutex<DanmakuManager>>,
    pub hotkey: Arc<Mutex<HotkeyManager>>,
    pub obs_overlay: Arc<Mutex<OBSOverlayServer>>,
}

pub fn play_next_song(app: &tauri::AppHandle) -> Result<Option<SongRequest>, String> {
    let state = app.state::<AppState>();
    let Some(request) = state.queue.next() else {
        let _ = app.emit("log-event", "队列为空，没有歌曲可播放");
        return Ok(None);
    };

    if let Err(error) = state
        .selector
        .lock()
        .map_err(|_| "切歌器锁失败".to_string())?
        .change_song(request.song_id)
    {
        state.queue.requeue_front(request);
        return Err(error);
    }

    state.queue.complete(request.clone());
    let _ = app.emit("queue-updated", state.queue.snapshot());
    let _ = app.emit("log-event", format!("已切换到歌曲: {}", request.song_name));
    Ok(Some(request))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Err(error) = play_next_song(app) {
                            let _ = app.emit("log-event", format!("快捷键切歌失败: {error}"));
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("config.json");
            let config = load_config_with_dev_fallback(&config_path);
            if !config_path.exists() {
                config.save(&config_path).map_err(std::io::Error::other)?;
            }

            let resource_dir = app.path().resource_dir().ok();
            let data_dir = resolve_data_dir(&config.data_dir, resource_dir.as_deref());
            let database = SongDatabase::load_with_chinese_names(&data_dir).unwrap_or_default();
            let searcher = SongSearcher::from_database(&database, &data_dir);
            let queue = Arc::new(SongQueue::new(
                config.max_queue_size,
                config.allow_duplicates,
            ));

            let hotkey = HotkeyManager::new(config.hotkey.clone());
            let obs_overlay = OBSOverlayServer::new(
                config.obs_overlay_host.clone(),
                config.obs_overlay_port,
                config.obs_overlay_title.clone(),
                queue.clone(),
            );

            app.manage(AppState {
                config_path,
                data_dir: Arc::new(RwLock::new(data_dir)),
                config: Arc::new(RwLock::new(config)),
                queue,
                database: Arc::new(RwLock::new(database)),
                searcher: Arc::new(RwLock::new(searcher)),
                selector: Arc::new(Mutex::new(SongSelector::new())),
                danmaku: Arc::new(Mutex::new(DanmakuManager::default())),
                hotkey: Arc::new(Mutex::new(hotkey)),
                obs_overlay: Arc::new(Mutex::new(obs_overlay)),
            });

            let state = app.state::<AppState>();
            if let Err(error) = state
                .hotkey
                .lock()
                .map_err(|_| std::io::Error::other("快捷键管理器锁失败"))?
                .register(app.handle())
            {
                let _ = app.emit("log-event", format!("注册快捷键失败: {error}"));
            }
            let config = state
                .config
                .read()
                .map_err(|_| std::io::Error::other("读取配置锁失败"))?
                .clone();
            if config.obs_overlay_enabled {
                if let Err(error) = state
                    .obs_overlay
                    .lock()
                    .map_err(|_| std::io::Error::other("OBS 管理器锁失败"))?
                    .start(app.handle())
                {
                    let _ = app.emit("log-event", format!("启动 OBS 覆盖层失败: {error}"));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::validate_config,
            commands::get_all_songs,
            commands::reload_database,
            commands::rebuild_database,
            commands::search_songs,
            commands::search_songs_by_author,
            commands::debug_song_request,
            commands::debug_enqueue_song,
            commands::get_queue,
            commands::get_queue_history,
            commands::remove_from_queue,
            commands::clear_queue,
            commands::next_song,
            commands::change_song,
            commands::get_game_connection_status,
            commands::reconnect_game,
            commands::start_danmaku,
            commands::stop_danmaku,
            commands::get_danmaku_status,
            commands::register_hotkey,
            commands::unregister_hotkey,
            commands::get_hotkey_status,
            commands::start_obs_overlay,
            commands::stop_obs_overlay,
            commands::get_obs_overlay_status,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");
}

fn load_config_with_dev_fallback(config_path: &Path) -> Config {
    if config_path.exists() {
        return Config::load_or_default(config_path);
    }

    for candidate in dev_config_candidates() {
        if candidate.exists() {
            return Config::load_or_default(&candidate);
        }
    }

    Config::default()
}

fn resolve_data_dir(configured: &str, resource_dir: Option<&Path>) -> PathBuf {
    let configured_path = PathBuf::from(configured);
    if configured_path.is_absolute() && configured_path.exists() {
        return configured_path;
    }

    let mut candidates = Vec::new();
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(configured));
        candidates.push(current_dir.join("..").join(configured));
        candidates.push(current_dir.join("..").join("..").join(configured));
    }
    if let Some(resource_dir) = resource_dir {
        candidates.push(resource_dir.join(configured));
        candidates.push(resource_dir.join("Data"));
        candidates.push(resource_dir.to_path_buf());
        candidates.push(resource_dir.join("_up_").join("Data"));
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.join("song_db.json").exists())
    {
        return candidate.clone();
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .unwrap_or(configured_path)
}

fn dev_config_candidates() -> Vec<PathBuf> {
    let Ok(current_dir) = std::env::current_dir() else {
        return Vec::new();
    };
    vec![
        current_dir.join("config.json"),
        current_dir.join("..").join("config.json"),
        current_dir.join("..").join("..").join("config.json"),
    ]
}
