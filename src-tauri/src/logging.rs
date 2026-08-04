use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use chrono::Local;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

/// 统一日志入口：向前端日志页发送事件，并按配置追加写入本地日志文件。
/// 配置未开启或写入失败时静默跳过，不影响正常的日志事件。
pub(crate) fn emit_log(app: &AppHandle, message: impl AsRef<str>) {
    let message = message.as_ref();
    let _ = app.emit("log-event", message);

    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let enabled = state
        .config
        .read()
        .map(|config| config.log_to_file)
        .unwrap_or(false);
    if !enabled {
        return;
    }
    let Ok(data_dir) = state.data_dir.read().map(|dir| dir.clone()) else {
        return;
    };
    append_to_log_file(&data_dir, message);
}

/// 追加一行日志到 {data_dir}/logs/diva-live-helper-YYYYMMDD.log（按日期分文件）
fn append_to_log_file(data_dir: &Path, message: &str) {
    let now = Local::now();
    let log_dir = data_dir.join("logs");
    if std::fs::create_dir_all(&log_dir).is_err() {
        return;
    }
    let path = log_dir.join(format!("diva-live-helper-{}.log", now.format("%Y%m%d")));
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {message}", now.format("%Y-%m-%d %H:%M:%S"));
    }
}

#[cfg(test)]
mod tests {
    use super::append_to_log_file;

    // ----------------- append_to_log_file -----------------

    #[test]
    fn append_writes_timestamped_lines_to_daily_file() {
        let dir = std::env::temp_dir().join(format!("diva_log_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        append_to_log_file(&dir, "测试日志");
        append_to_log_file(&dir, "第二条");

        // 同一天只生成一个按日期命名的文件
        let log_dir = dir.join("logs");
        let entries: Vec<_> = std::fs::read_dir(&log_dir)
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
        let file_name = entries[0].file_name().into_string().unwrap();
        assert!(file_name.starts_with("diva-live-helper-"));
        assert!(file_name.ends_with(".log"));

        let content = std::fs::read_to_string(entries[0].path()).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        // 每行以 [YYYY-MM-DD HH:MM:SS] 时间戳开头
        for line in &lines {
            assert!(line.starts_with('['));
            assert!(line.len() > 21);
        }
        assert!(lines[0].ends_with("测试日志"));
        assert!(lines[1].ends_with("第二条"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn append_creates_logs_dir_when_missing() {
        let dir = std::env::temp_dir().join(format!("diva_log_test_mkdir_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        append_to_log_file(&dir, "hello");

        assert!(dir.join("logs").is_dir());
        std::fs::remove_dir_all(&dir).ok();
    }
}
