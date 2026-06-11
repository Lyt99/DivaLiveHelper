use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub room_id: u64,
    pub hotkey: String,
    pub data_dir: String,
    pub mods_dir: String,
    pub auto_play_next: bool,
    pub auto_play_interval: u32,
    pub max_queue_size: usize,
    pub allow_duplicates: bool,
    pub obs_overlay_enabled: bool,
    pub obs_overlay_host: String,
    pub obs_overlay_port: u16,
    pub obs_overlay_title: String,
    pub song_command_prefix: String,
    pub sessdata: String,
    pub fetch_chinese_names: bool,
    pub http_proxy: String,
    pub default_search_difficulty: String,
    pub difficulty_tolerance: f32,
    pub llm_enabled: bool,
    pub llm_api_key: String,
    pub llm_base_url: String,
    pub llm_model: String,
    pub config_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            room_id: 0,
            hotkey: "ctrl+shift+n".to_string(),
            data_dir: "Data".to_string(),
            mods_dir: String::new(),
            auto_play_next: false,
            auto_play_interval: 300,
            max_queue_size: 50,
            allow_duplicates: false,
            obs_overlay_enabled: true,
            obs_overlay_host: "127.0.0.1".to_string(),
            obs_overlay_port: 8765,
            obs_overlay_title: "点歌队列".to_string(),
            song_command_prefix: "点歌".to_string(),
            sessdata: String::new(),
            fetch_chinese_names: false,
            http_proxy: String::new(),
            default_search_difficulty: "extreme".to_string(),
            difficulty_tolerance: 0.5,
            llm_enabled: false,
            llm_api_key: String::new(),
            llm_base_url: "https://api.deepseek.com".to_string(),
            llm_model: "deepseek-chat".to_string(),
            config_file: "config.json".to_string(),
        }
    }
}

impl Config {
    pub fn load_or_default(path: &Path) -> Self {
        match Self::load(path) {
            Ok(config) => config,
            Err(_) => Self::default(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|error| format!("读取配置失败: {error}"))?;
        serde_json::from_str(&content).map_err(|error| format!("解析配置失败: {error}"))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建配置目录失败: {error}"))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|error| format!("序列化配置失败: {error}"))?;
        fs::write(path, content).map_err(|error| format!("保存配置失败: {error}"))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.room_id == 0 {
            return Err("错误: 未设置直播间ID".to_string());
        }
        if self.hotkey.trim().is_empty() {
            return Err("错误: 未设置快捷键".to_string());
        }
        if self.obs_overlay_enabled
            && self.obs_overlay_host != "127.0.0.1"
            && self.obs_overlay_host != "localhost"
        {
            return Err("错误: OBS点歌队列组件仅支持本机地址 127.0.0.1 或 localhost".to_string());
        }
        Ok(())
    }
}
