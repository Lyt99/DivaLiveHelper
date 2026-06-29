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
    pub difficulty_fallback: String,
    pub llm_enabled: bool,
    pub llm_api_key: String,
    pub llm_base_url: String,
    pub llm_model: String,
    pub llm_max_tokens: Option<u32>,
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
            difficulty_fallback: "easier".to_string(),
            llm_enabled: false,
            llm_api_key: String::new(),
            llm_base_url: "https://api.deepseek.com".to_string(),
            llm_model: "deepseek-chat".to_string(),
            llm_max_tokens: Some(150),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> Config {
        Config {
            room_id: 12345,
            hotkey: "ctrl+shift+n".to_string(),
            ..Config::default()
        }
    }

    // ----------------- validate -----------------

    #[test]
    fn validate_passes_for_valid_config() {
        let config = valid_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_zero_room_id() {
        let mut config = valid_config();
        config.room_id = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_hotkey() {
        let mut config = valid_config();
        config.hotkey = "   ".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_remote_obs_host_when_enabled() {
        let mut config = valid_config();
        config.obs_overlay_enabled = true;
        config.obs_overlay_host = "192.168.1.1".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_accepts_localhost_obs_host() {
        let mut config = valid_config();
        config.obs_overlay_enabled = true;
        config.obs_overlay_host = "localhost".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_127_0_0_1_obs_host() {
        let mut config = valid_config();
        config.obs_overlay_enabled = true;
        config.obs_overlay_host = "127.0.0.1".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_skips_obs_host_check_when_disabled() {
        let mut config = valid_config();
        config.obs_overlay_enabled = false;
        config.obs_overlay_host = "0.0.0.0".to_string();
        assert!(config.validate().is_ok());
    }

    // ----------------- defaults -----------------

    #[test]
    fn default_config_has_expected_values() {
        let config = Config::default();
        assert_eq!(config.room_id, 0);
        assert_eq!(config.hotkey, "ctrl+shift+n");
        assert_eq!(config.data_dir, "Data");
        assert_eq!(config.max_queue_size, 50);
        assert!(!config.allow_duplicates);
        assert_eq!(config.default_search_difficulty, "extreme");
        assert_eq!(config.difficulty_tolerance, 0.5);
        assert_eq!(config.difficulty_fallback, "easier");
        assert_eq!(config.obs_overlay_port, 8765);
        assert_eq!(config.song_command_prefix, "点歌");
        assert!(!config.llm_enabled);
        assert_eq!(config.llm_model, "deepseek-chat");
    }

    // ----------------- load / save round-trip -----------------

    #[test]
    fn save_then_load_preserves_all_fields() {
        let dir = std::env::temp_dir().join("diva_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config_roundtrip.json");

        let original = Config {
            room_id: 99999,
            hotkey: "alt+f12".to_string(),
            data_dir: "MyData".to_string(),
            mods_dir: "C:/mods".to_string(),
            auto_play_next: true,
            auto_play_interval: 120,
            max_queue_size: 20,
            allow_duplicates: true,
            obs_overlay_enabled: false,
            obs_overlay_host: "localhost".to_string(),
            obs_overlay_port: 9999,
            obs_overlay_title: "Test Queue".to_string(),
            song_command_prefix: "播".to_string(),
            sessdata: "secret_sessdata".to_string(),
            fetch_chinese_names: true,
            http_proxy: "http://proxy:8080".to_string(),
            default_search_difficulty: "exextreme".to_string(),
            difficulty_tolerance: 1.0,
            difficulty_fallback: "harder".to_string(),
            llm_enabled: true,
            llm_api_key: "sk-test".to_string(),
            llm_base_url: "https://api.example.com".to_string(),
            llm_model: "gpt-4o".to_string(),
            llm_max_tokens: Some(300),
            config_file: "custom.json".to_string(),
        };

        original.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();

        assert_eq!(loaded.room_id, 99999);
        assert_eq!(loaded.hotkey, "alt+f12");
        assert_eq!(loaded.data_dir, "MyData");
        assert_eq!(loaded.mods_dir, "C:/mods");
        assert!(loaded.auto_play_next);
        assert_eq!(loaded.auto_play_interval, 120);
        assert_eq!(loaded.max_queue_size, 20);
        assert!(loaded.allow_duplicates);
        assert!(!loaded.obs_overlay_enabled);
        assert_eq!(loaded.obs_overlay_host, "localhost");
        assert_eq!(loaded.obs_overlay_port, 9999);
        assert_eq!(loaded.obs_overlay_title, "Test Queue");
        assert_eq!(loaded.song_command_prefix, "播");
        assert_eq!(loaded.sessdata, "secret_sessdata");
        assert!(loaded.fetch_chinese_names);
        assert_eq!(loaded.http_proxy, "http://proxy:8080");
        assert_eq!(loaded.default_search_difficulty, "exextreme");
        assert_eq!(loaded.difficulty_tolerance, 1.0);
        assert_eq!(loaded.difficulty_fallback, "harder");
        assert!(loaded.llm_enabled);
        assert_eq!(loaded.llm_api_key, "sk-test");
        assert_eq!(loaded.llm_base_url, "https://api.example.com");
        assert_eq!(loaded.llm_model, "gpt-4o");
        assert_eq!(loaded.llm_max_tokens, Some(300));
        assert_eq!(loaded.config_file, "custom.json");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_returns_err_for_nonexistent_file() {
        let path = std::env::temp_dir().join("diva_config_nonexistent_12345.json");
        let _ = std::fs::remove_file(&path);
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn load_or_default_falls_back_on_missing_file() {
        let path = std::env::temp_dir().join("diva_config_missing_67890.json");
        let _ = std::fs::remove_file(&path);
        let config = Config::load_or_default(&path);
        assert_eq!(config.room_id, 0); // default
    }

    #[test]
    fn load_or_default_loads_when_file_exists() {
        let dir = std::env::temp_dir().join("diva_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config_load_or_default.json");

        let config = Config {
            room_id: 42,
            ..Config::default()
        };
        config.save(&path).unwrap();

        let loaded = Config::load_or_default(&path);
        assert_eq!(loaded.room_id, 42);

        let _ = std::fs::remove_file(&path);
    }

    // ----------------- serde with #[serde(default)] -----------------

    #[test]
    fn config_deserializes_with_missing_fields_via_serde_default() {
        // config.json 只包含部分字段时，缺失字段应使用 Default
        let json = r#"{"room_id": 100, "hotkey": "ctrl+n"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.room_id, 100);
        assert_eq!(config.hotkey, "ctrl+n");
        // 缺失字段用默认值
        assert_eq!(config.data_dir, "Data");
        assert_eq!(config.max_queue_size, 50);
        assert_eq!(config.default_search_difficulty, "extreme");
    }
}
