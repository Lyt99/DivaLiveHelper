use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongEntry {
    pub pv_id: u32,
    pub name: String,
    pub name_en: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub difficulty: HashMap<String, f32>,
    pub source: String,
    #[serde(skip)]
    pub name_zh: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongDatabaseFile {
    pub version: u32,
    pub songs: HashMap<String, SongEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct SongDatabase {
    pub songs: HashMap<u32, SongEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BaseSongDatabaseFile {
    pub version: u32,
    pub songs: HashMap<String, BaseSongEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BaseSongEntry {
    pub pv_id: u32,
    pub name: String,
    pub name_en: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub difficulty: HashMap<String, f32>,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SongInfo {
    pub pv_id: u32,
    pub name: String,
    pub name_en: Option<String>,
    pub name_zh: Option<String>,
    pub authors: Vec<String>,
    pub difficulty: HashMap<String, f32>,
    pub source: Option<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ChineseNameFile {
    entries: HashMap<String, ChineseNameEntry>,
}

#[derive(Debug, Deserialize)]
struct ChineseNameEntry {
    name_zh: String,
}

impl SongDatabase {
    pub fn load_with_chinese_names(data_dir: &Path) -> Result<Self, String> {
        let mut database = Self::load(&data_dir.join("song_db.json"))?;
        if let Ok(chinese_names) = load_chinese_names(&data_dir.join("song_name_zh.json")) {
            database.apply_chinese_names(&chinese_names);
        }
        Ok(database)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|error| format!("读取歌曲数据库失败: {error}"))?;
        let parsed: SongDatabaseFile = serde_json::from_str(&content)
            .map_err(|error| format!("解析歌曲数据库失败: {error}"))?;
        let mut songs = HashMap::new();
        for (key, mut entry) in parsed.songs {
            let pv_id = key.parse::<u32>().unwrap_or(entry.pv_id);
            entry.pv_id = pv_id;
            songs.insert(pv_id, entry);
        }
        Ok(Self { songs })
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建歌曲库目录失败: {error}"))?;
        }
        let songs = self
            .songs
            .iter()
            .map(|(pv_id, entry)| (pv_id.to_string(), entry.clone()))
            .collect();
        let content = serde_json::to_string_pretty(&SongDatabaseFile { version: 1, songs })
            .map_err(|error| format!("序列化歌曲数据库失败: {error}"))?;
        fs::write(path, content).map_err(|error| format!("保存歌曲数据库失败: {error}"))
    }

    pub fn import_from_pvdb(&mut self, path: &Path, source: &str) -> Result<usize, String> {
        let content =
            fs::read_to_string(path).map_err(|error| format!("读取 pv_db 失败: {error}"))?;
        let mut raw_entries: HashMap<u32, HashMap<String, String>> = HashMap::new();
        for line in content.lines().map(str::trim) {
            if line.is_empty()
                || line.starts_with('#')
                || !line.contains('=')
                || !line.contains('.')
            {
                continue;
            }
            let Some((left, value)) = line.split_once('=') else {
                continue;
            };
            let Some((pv_part, key)) = left.split_once('.') else {
                continue;
            };
            let Some(id_part) = pv_part.strip_prefix("pv_") else {
                continue;
            };
            let Ok(pv_id) = id_part.parse::<u32>() else {
                continue;
            };
            raw_entries
                .entry(pv_id)
                .or_default()
                .insert(key.to_string(), value.to_string());
        }

        let mut imported = 0;
        for (pv_id, fields) in raw_entries {
            let name = fields.get("song_name").cloned().unwrap_or_default();
            let name_en = fields.get("song_name_en").cloned().unwrap_or_default();
            let authors = fields
                .get("songinfo.music")
                .or_else(|| fields.get("songinfo_en.music"))
                .map(|author| author.trim().to_string())
                .filter(|author| !author.is_empty())
                .map(|author| vec![author])
                .unwrap_or_default();
            let difficulty = parse_difficulty(&fields);
            self.songs
                .entry(pv_id)
                .and_modify(|entry| {
                    if !name.is_empty() {
                        entry.name = name.clone();
                    }
                    if !name_en.is_empty() {
                        entry.name_en = name_en.clone();
                    }
                    if !authors.is_empty() {
                        entry.authors = authors.clone();
                    }
                    if !difficulty.is_empty() {
                        entry.difficulty = difficulty.clone();
                    }
                    entry.source = source.to_string();
                })
                .or_insert_with(|| SongEntry {
                    pv_id,
                    name: name.clone(),
                    name_en: name_en.clone(),
                    aliases: Vec::new(),
                    authors: authors.clone(),
                    difficulty: difficulty.clone(),
                    source: source.to_string(),
                    name_zh: None,
                });
            imported += 1;
        }
        Ok(imported)
    }

    pub fn import_base_json(&mut self, path: &Path) -> Result<usize, String> {
        let content =
            fs::read_to_string(path).map_err(|error| format!("读取内置基础歌曲库失败: {error}"))?;
        let parsed: BaseSongDatabaseFile = serde_json::from_str(&content)
            .map_err(|error| format!("解析内置基础歌曲库失败: {error}"))?;
        if parsed.version == 0 {
            return Err("内置基础歌曲库版本无效".to_string());
        }

        let mut imported = 0;
        for (key, base_entry) in parsed.songs {
            let pv_id = key.parse::<u32>().unwrap_or(base_entry.pv_id);
            let source = if base_entry.source.trim().is_empty() {
                "base".to_string()
            } else {
                base_entry.source
            };
            self.songs.insert(
                pv_id,
                SongEntry {
                    pv_id,
                    name: base_entry.name,
                    name_en: base_entry.name_en,
                    aliases: base_entry.aliases,
                    authors: base_entry.authors,
                    difficulty: base_entry.difficulty,
                    source,
                    name_zh: base_entry.name_zh,
                },
            );
            imported += 1;
        }
        Ok(imported)
    }

    pub fn import_aliases(&mut self, path: &Path) -> Result<usize, String> {
        if !path.exists() {
            return Ok(0);
        }
        let content =
            fs::read_to_string(path).map_err(|error| format!("读取别名文件失败: {error}"))?;
        let aliases: HashMap<String, String> =
            serde_json::from_str(&content).map_err(|error| format!("解析别名文件失败: {error}"))?;
        let mut name_index = HashMap::new();
        for (pv_id, entry) in &self.songs {
            if !entry.name.is_empty() {
                name_index.insert(entry.name.to_lowercase(), *pv_id);
            }
            if !entry.name_en.is_empty() {
                name_index.insert(entry.name_en.to_lowercase(), *pv_id);
            }
        }

        let mut imported = 0;
        for (alias, target) in aliases {
            if let Some(pv_id) = name_index.get(&target.to_lowercase()) {
                if let Some(entry) = self.songs.get_mut(pv_id) {
                    if !entry.aliases.iter().any(|existing| existing == &alias) {
                        entry.aliases.push(alias);
                        imported += 1;
                    }
                }
            }
        }
        Ok(imported)
    }

    pub fn remove_source(&mut self, source: &str) -> usize {
        let before = self.songs.len();
        self.songs.retain(|_, entry| entry.source != source);
        before - self.songs.len()
    }

    pub fn remove_unnamed(&mut self) -> usize {
        let before = self.songs.len();
        self.songs
            .retain(|_, entry| !entry.name.is_empty() || !entry.name_en.is_empty());
        before - self.songs.len()
    }

    pub fn source_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        for entry in self.songs.values() {
            let source = if entry.source.starts_with("mod:") {
                "mod"
            } else {
                &entry.source
            };
            *stats.entry(source.to_string()).or_insert(0) += 1;
        }
        stats
    }

    pub fn apply_chinese_names(&mut self, chinese_names: &HashMap<String, String>) {
        for entry in self.songs.values_mut() {
            if let Some(name_zh) = chinese_names.get(&entry.name) {
                entry.name_zh = Some(name_zh.clone());
            }
        }
    }

    pub fn to_song_infos(&self) -> Vec<SongInfo> {
        let mut songs: Vec<SongInfo> = self
            .songs
            .values()
            .map(|entry| SongInfo {
                pv_id: entry.pv_id,
                name: entry.name.clone(),
                name_en: non_empty(&entry.name_en),
                name_zh: entry.name_zh.clone(),
                authors: entry.authors.clone(),
                difficulty: entry.difficulty.clone(),
                source: non_empty(&entry.source),
                aliases: entry.aliases.clone(),
            })
            .collect();
        songs.sort_by_key(|song| song.pv_id);
        songs
    }
}

fn parse_difficulty(fields: &HashMap<String, String>) -> HashMap<String, f32> {
    let mut difficulty = HashMap::new();
    for (db_key, our_key) in [
        ("easy", "easy"),
        ("normal", "normal"),
        ("hard", "hard"),
        ("extreme", "extreme"),
    ] {
        if let Some(level) = fields
            .get(&format!("difficulty.{db_key}.0.level"))
            .and_then(|value| parse_level(value))
        {
            difficulty.insert(our_key.to_string(), level);
        }
    }
    if let Some(level) = fields
        .get("difficulty.extreme.1.level")
        .and_then(|value| parse_level(value))
    {
        difficulty.insert("exextreme".to_string(), level);
    }
    difficulty
}

fn parse_level(value: &str) -> Option<f32> {
    let parts: Vec<&str> = value.split('_').collect();
    if parts.len() < 4 || parts[0] != "PV" || parts[1] != "LV" {
        return None;
    }
    let integer = parts[2].parse::<u32>().ok()?;
    let decimal = parts[3].parse::<u32>().ok()?;
    Some(format!("{integer}.{decimal}").parse::<f32>().ok()?)
}

fn load_chinese_names(path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        fs::read_to_string(path).map_err(|error| format!("读取中文名数据库失败: {error}"))?;
    let parsed: ChineseNameFile =
        serde_json::from_str(&content).map_err(|error| format!("解析中文名数据库失败: {error}"))?;
    Ok(parsed
        .entries
        .into_iter()
        .map(|(name, entry)| (name, entry.name_zh))
        .collect())
}

fn non_empty(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
