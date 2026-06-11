use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[derive(Debug, Clone, Serialize)]
pub struct HotkeyStatus {
    pub registered: bool,
    pub hotkey: String,
    pub message: String,
}

#[derive(Debug)]
pub struct HotkeyManager {
    hotkey: String,
    registered: bool,
    message: String,
}

impl HotkeyManager {
    pub fn new(hotkey: String) -> Self {
        Self {
            hotkey,
            registered: false,
            message: "快捷键尚未注册".to_string(),
        }
    }

    pub fn status(&self) -> HotkeyStatus {
        HotkeyStatus {
            registered: self.registered,
            hotkey: self.hotkey.clone(),
            message: self.message.clone(),
        }
    }

    pub fn register(&mut self, app: &AppHandle) -> Result<(), String> {
        let normalized = normalize_hotkey(&self.hotkey)?;
        app.global_shortcut()
            .unregister_all()
            .map_err(|error| format!("注销旧快捷键失败: {error}"))?;
        app.global_shortcut()
            .register(normalized.as_str())
            .map_err(|error| format!("注册快捷键失败: {error}"))?;
        self.hotkey = normalized;
        self.registered = true;
        self.message = format!("快捷键已注册: {}", self.hotkey);
        Ok(())
    }

    pub fn update_and_register(&mut self, app: &AppHandle, hotkey: String) -> Result<(), String> {
        self.hotkey = hotkey;
        self.register(app)
    }

    pub fn unregister(&mut self, app: &AppHandle) -> Result<(), String> {
        app.global_shortcut()
            .unregister_all()
            .map_err(|error| format!("注销快捷键失败: {error}"))?;
        self.registered = false;
        self.message = "快捷键已停止".to_string();
        Ok(())
    }
}

fn normalize_hotkey(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("快捷键不能为空".to_string());
    }

    let parts: Vec<String> = value
        .split('+')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(normalize_part)
        .collect();

    if parts.len() < 2 {
        return Err("快捷键至少需要一个修饰键和一个按键，例如 Ctrl+Shift+N".to_string());
    }
    Ok(parts.join("+"))
}

fn normalize_part(part: &str) -> String {
    match part.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => "ctrl".to_string(),
        "cmd" | "command" | "meta" | "super" => "command".to_string(),
        "cmdorctrl" | "commandorcontrol" => "commandorcontrol".to_string(),
        "shift" => "shift".to_string(),
        "alt" | "option" => "alt".to_string(),
        "plus" => "plus".to_string(),
        key => key.to_string(),
    }
}
