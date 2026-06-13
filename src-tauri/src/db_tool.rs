use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::song_db::SongDatabase;

#[derive(Debug, Clone, Serialize)]
pub struct RebuildReport {
    pub data_dir: String,
    pub mods_dir: Option<String>,
    pub total: usize,
    pub base_imported: usize,
    pub mods_imported: usize,
    pub mods_scanned: usize,
    pub aliases_imported: usize,
    pub chinese_names_merged: usize,
    pub removed_mdata: usize,
    pub removed_unnamed: usize,
    pub sources: HashMap<String, usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ChineseNameFile {
    version: u32,
    entries: HashMap<String, ChineseNameEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ChineseNameEntry {
    name_zh: String,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    name_en: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    candidate: String,
    #[serde(default)]
    evidence: String,
}

pub fn rebuild_database(data_dir: &Path, mods_dir: Option<&Path>) -> Result<RebuildReport, String> {
    if !data_dir.exists() {
        return Err(format!("数据目录不存在: {}", data_dir.display()));
    }

    let db_path = data_dir.join("song_db.json");
    let mut database = if db_path.exists() {
        SongDatabase::load(&db_path).unwrap_or_default()
    } else {
        SongDatabase::default()
    };

    let base_db = data_dir.join("base_song_db.json");
    let base_imported = if base_db.exists() {
        database.import_base_json(&base_db)?
    } else {
        let pv_db = data_dir.join("pv_db.txt");
        if pv_db.exists() {
            database.import_from_pvdb(&pv_db, "base")?
        } else {
            0
        }
    };

    let mut mods_imported = 0;
    let mut mods_scanned = 0;
    if let Some(mods_dir) = mods_dir.filter(|path| path.exists()) {
        for entry in
            std::fs::read_dir(mods_dir).map_err(|error| format!("读取 MOD 目录失败: {error}"))?
        {
            let entry = entry.map_err(|error| format!("读取 MOD 条目失败: {error}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let mod_pv_db = path.join("rom").join("mod_pv_db.txt");
            if !mod_pv_db.exists() {
                continue;
            }
            let mod_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown");
            mods_imported += database.import_from_pvdb(&mod_pv_db, &format!("mod:{mod_name}"))?;
            mods_scanned += 1;
        }
    }

    let removed_mdata = database.remove_source("mdata");
    let removed_unnamed = database.remove_unnamed();
    let aliases_imported = database.import_aliases(&data_dir.join("AnotherSongName.json"))?;
    let chinese_names_merged = merge_chinese_names(data_dir, &database)?;
    database.save(&db_path)?;

    Ok(RebuildReport {
        data_dir: data_dir.display().to_string(),
        mods_dir: mods_dir
            .map(PathBuf::from)
            .map(|path| path.display().to_string()),
        total: database.songs.len(),
        base_imported,
        mods_imported,
        mods_scanned,
        aliases_imported,
        chinese_names_merged,
        removed_mdata,
        removed_unnamed,
        sources: database.source_stats(),
    })
}

fn merge_chinese_names(data_dir: &Path, database: &SongDatabase) -> Result<usize, String> {
    let zh_path = data_dir.join("song_name_zh.json");
    let mut zh = if zh_path.exists() {
        let content = std::fs::read_to_string(&zh_path)
            .map_err(|error| format!("读取中文名数据库失败: {error}"))?;
        serde_json::from_str::<ChineseNameFile>(&content).unwrap_or_default()
    } else {
        ChineseNameFile {
            version: 3,
            entries: HashMap::new(),
        }
    };
    zh.version = 3;

    let cache = load_name_cache(&data_dir.join("song_name_cache.json"))?;
    let mut merged = 0;
    for entry in database.songs.values() {
        if entry.name.is_empty() || zh.entries.contains_key(&entry.name) {
            continue;
        }
        let Some((name_zh, evidence)) = entry
            .name_zh
            .as_ref()
            .filter(|name| !name.trim().is_empty())
            .map(|name| (name.clone(), "base-song-db".to_string()))
            .or_else(|| {
                cache
                    .get(&entry.name)
                    .or_else(|| cache.get(&entry.name_en))
                    .map(|name| (name.clone(), "song-name-cache".to_string()))
            })
        else {
            continue;
        };
        zh.entries.insert(
            entry.name.clone(),
            ChineseNameEntry {
                name_zh,
                status: "auto".to_string(),
                source: entry.source.clone(),
                name_en: entry.name_en.clone(),
                author: entry.authors.join(","),
                candidate: String::new(),
                evidence,
            },
        );
        merged += 1;
    }

    if let Some(parent) = zh_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("创建中文名目录失败: {error}"))?;
    }
    let content = serde_json::to_string_pretty(&zh)
        .map_err(|error| format!("序列化中文名数据库失败: {error}"))?;
    std::fs::write(&zh_path, content).map_err(|error| format!("保存中文名数据库失败: {error}"))?;
    Ok(merged)
}

fn load_name_cache(path: &Path) -> Result<HashMap<String, String>, String> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("读取旧版中文名缓存失败: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("解析旧版中文名缓存失败: {error}"))
}

fn default_status() -> String {
    "auto".to_string()
}
