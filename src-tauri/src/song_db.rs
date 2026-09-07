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
    pub mod_name: Option<String>,
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

        // Start from the embedded base Chinese name DB
        const EMBEDDED_BASE_ZH_DB: &str = include_str!("../../Data/song_name_zh.json");
        let base_zh: HashMap<String, String> = serde_json::from_str::<ChineseNameFile>(EMBEDDED_BASE_ZH_DB)
            .map(|file| file.entries.into_iter().map(|(name, entry)| (name, entry.name_zh)).collect())
            .unwrap_or_default();
        database.apply_chinese_names(&base_zh);

        // Override with the on-disk file if present
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

    pub fn import_base_json_str(&mut self, content: &str) -> Result<usize, String> {
        let parsed: BaseSongDatabaseFile = serde_json::from_str(content)
            .map_err(|error| format!("解析内置基础歌曲库失败: {error}"))?;
        if parsed.version == 0 {
            return Err("内置基础歌曲库版本无效".to_string());
        }

        // 内置库是完整的官方曲目清单，先移除旧官方记录，避免已剔除的曲目残留。
        // 保留 MOD 曲目；后续扫描仍可按原有顺序覆盖官方曲目。
        self.songs
            .retain(|_, entry| !matches!(entry.source.as_str(), "base" | "dlc"));

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

    pub fn import_base_json(&mut self, path: &Path) -> Result<usize, String> {
        let content =
            fs::read_to_string(path).map_err(|error| format!("读取内置基础歌曲库失败: {error}"))?;
        self.import_base_json_str(&content)
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
                mod_name: None,
                aliases: entry.aliases.clone(),
            })
            .collect();
        songs.sort_by_key(|song| song.pv_id);
        songs
    }
}

#[derive(Deserialize)]
struct ModMetadata {
    name: String,
}

pub fn resolve_mod_names(songs: &mut [SongInfo], mods_dir: Option<&Path>) {
    let mut names = HashMap::new();
    for song in songs {
        let Some(folder) = song.source.as_deref().and_then(|source| source.strip_prefix("mod:")) else {
            song.mod_name = None;
            continue;
        };
        let name = names.entry(folder).or_insert_with(|| {
            mods_dir
                .and_then(|root| load_mod_name(&root.join(folder)))
                .unwrap_or_else(|| folder.to_string())
        });
        song.mod_name = Some(name.clone());
    }
}

fn load_mod_name(mod_dir: &Path) -> Option<String> {
    fs::read_to_string(mod_dir.join("mod.json"))
        .ok()
        .and_then(|content| serde_json::from_str::<ModMetadata>(&content).ok())
        .and_then(|metadata| non_empty(metadata.name.trim()))
        .or_else(|| {
            fs::read_to_string(mod_dir.join("config.toml"))
                .ok()
                .and_then(|content| toml::from_str::<ModMetadata>(&content).ok())
                .and_then(|metadata| non_empty(metadata.name.trim()))
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    // ----------------- parse_level -----------------

    #[test]
    fn parse_level_valid_pv_lv_format() {
        assert_eq!(parse_level("PV_LV_10_50"), Some(10.5));
        assert_eq!(parse_level("PV_LV_8_0"), Some(8.0));
        assert_eq!(parse_level("PV_LV_0_0"), Some(0.0));
        assert_eq!(parse_level("PV_LV_12_25"), Some(12.25));
    }

    #[test]
    fn parse_level_rejects_wrong_prefix() {
        assert_eq!(parse_level("XX_LV_10_50"), None);
        assert_eq!(parse_level("PV_XX_10_50"), None);
    }

    #[test]
    fn parse_level_rejects_insufficient_parts() {
        assert_eq!(parse_level(""), None);
        assert_eq!(parse_level("PV"), None);
        assert_eq!(parse_level("PV_LV"), None);
        assert_eq!(parse_level("PV_LV_10"), None);
    }

    #[test]
    fn parse_level_rejects_non_numeric() {
        assert_eq!(parse_level("PV_LV_a_b"), None);
        assert_eq!(parse_level("PV_LV_10_x"), None);
    }

    // ----------------- parse_difficulty -----------------

    fn make_fields(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn parse_difficulty_extracts_all_base_tiers() {
        let fields = make_fields(&[
            ("difficulty.easy.0.level", "PV_LV_2_0"),
            ("difficulty.normal.0.level", "PV_LV_5_0"),
            ("difficulty.hard.0.level", "PV_LV_7_50"),
            ("difficulty.extreme.0.level", "PV_LV_9_0"),
        ]);
        let diff = parse_difficulty(&fields);
        assert_eq!(diff.get("easy"), Some(&2.0));
        assert_eq!(diff.get("normal"), Some(&5.0));
        assert_eq!(diff.get("hard"), Some(&7.5));
        assert_eq!(diff.get("extreme"), Some(&9.0));
        assert!(!diff.contains_key("exextreme"));
    }

    #[test]
    fn parse_difficulty_extracts_exextreme_from_extreme_1() {
        let fields = make_fields(&[
            ("difficulty.extreme.0.level", "PV_LV_9_0"),
            ("difficulty.extreme.1.level", "PV_LV_10_50"),
        ]);
        let diff = parse_difficulty(&fields);
        assert_eq!(diff.get("extreme"), Some(&9.0));
        assert_eq!(diff.get("exextreme"), Some(&10.5));
    }

    #[test]
    fn parse_difficulty_returns_empty_for_no_data() {
        let fields = HashMap::new();
        let diff = parse_difficulty(&fields);
        assert!(diff.is_empty());
    }

    #[test]
    fn parse_difficulty_skips_invalid_levels() {
        let fields = make_fields(&[
            ("difficulty.easy.0.level", "invalid"),
            ("difficulty.hard.0.level", "PV_LV_6_0"),
        ]);
        let diff = parse_difficulty(&fields);
        assert!(!diff.contains_key("easy"));
        assert_eq!(diff.get("hard"), Some(&6.0));
    }

    // ----------------- import_base_json_str -----------------

    #[test]
    fn import_base_json_str_imports_valid_entries() {
        let json = r#"{
            "version": 1,
            "songs": {
                "100": {"pv_id": 100, "name": "Song A", "name_en": "SongA", "source": "base"},
                "200": {"pv_id": 200, "name": "Song B", "name_en": "SongB", "source": "dlc"}
            }
        }"#;
        let mut db = SongDatabase::default();
        let count = db.import_base_json_str(json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(db.songs.len(), 2);
        assert_eq!(db.songs.get(&100).unwrap().name, "Song A");
        assert_eq!(db.songs.get(&200).unwrap().source, "dlc");
    }

    #[test]
    fn import_base_json_str_rejects_version_zero() {
        let json = r#"{"version": 0, "songs": {}}"#;
        let mut db = SongDatabase::default();
        assert!(db.import_base_json_str(json).is_err());
    }

    #[test]
    fn import_base_json_str_rejects_invalid_json() {
        let mut db = SongDatabase::default();
        assert!(db.import_base_json_str("not json").is_err());
    }

    #[test]
    fn import_base_json_str_parses_pv_id_from_key_when_field_missing() {
        // key 优先于 field 中的 pv_id
        let json = r#"{
            "version": 1,
            "songs": {
                "999": {"pv_id": 0, "name": "From Key", "name_en": "", "source": ""}
            }
        }"#;
        let mut db = SongDatabase::default();
        db.import_base_json_str(json).unwrap();
        assert!(db.songs.contains_key(&999));
        assert_eq!(db.songs.get(&999).unwrap().pv_id, 999);
    }

    #[test]
    fn import_base_json_str_falls_back_source_to_base_when_empty() {
        let json = r#"{
            "version": 1,
            "songs": {
                "1": {"pv_id": 1, "name": "X", "name_en": "", "source": "  "}
            }
        }"#;
        let mut db = SongDatabase::default();
        db.import_base_json_str(json).unwrap();
        assert_eq!(db.songs.get(&1).unwrap().source, "base");
    }

    #[test]
    fn import_base_json_str_preserves_difficulty_and_chinese_name() {
        let json = r#"{
            "version": 1,
            "songs": {
                "1": {
                    "pv_id": 1, "name": "Test", "name_en": "TestEN",
                    "source": "base",
                    "name_zh": "测试",
                    "difficulty": {"extreme": 8.5, "exextreme": 9.5}
                }
            }
        }"#;
        let mut db = SongDatabase::default();
        db.import_base_json_str(json).unwrap();
        let entry = db.songs.get(&1).unwrap();
        assert_eq!(entry.name_zh, Some("测试".to_string()));
        assert_eq!(entry.difficulty.get("extreme"), Some(&8.5));
        assert_eq!(entry.difficulty.get("exextreme"), Some(&9.5));
    }

    #[test]
    fn import_base_json_str_replaces_official_catalog_without_removing_mod_songs() {
        let mut db = SongDatabase::default();
        for (pv_id, source) in [(700, "base"), (701, "dlc"), (27, "mod:Restore Cut Songs")] {
            db.songs.insert(pv_id, SongEntry {
                pv_id,
                name: "Ievan Polkka".into(),
                source: source.into(),
                ..Default::default()
            });
        }
        let json = r#"{
            "version": 1,
            "songs": {
                "262": {
                    "pv_id": 262, "name": "ピアノ×フォルテ×スキャンダル",
                    "name_en": "Piano x Forte x Scandal", "source": "base"
                }
            }
        }"#;

        for _ in 0..2 {
            db.import_base_json_str(json).unwrap();
            let ids: Vec<_> = db.to_song_infos().iter().map(|song| song.pv_id).collect();
            assert_eq!(ids, vec![27, 262]);
            assert_eq!(db.songs[&27].source, "mod:Restore Cut Songs");
        }
    }

    #[test]
    fn import_base_json_str_keeps_existing_catalog_on_invalid_input() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry {
            pv_id: 1,
            name: "恋は戦争".into(),
            source: "base".into(),
            ..Default::default()
        });
        for json in ["not json", r#"{"version": 0, "songs": {}}"#] {
            assert!(db.import_base_json_str(json).is_err());
            assert_eq!(db.to_song_infos()[0].name, "恋は戦争");
        }
    }

    // ----------------- remove_source / remove_unnamed -----------------

    #[test]
    fn remove_source_removes_matching_entries() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry { pv_id: 1, name: "A".into(), source: "base".into(), ..Default::default() });
        db.songs.insert(2, SongEntry { pv_id: 2, name: "B".into(), source: "dlc".into(), ..Default::default() });
        db.songs.insert(3, SongEntry { pv_id: 3, name: "C".into(), source: "base".into(), ..Default::default() });
        let removed = db.remove_source("base");
        assert_eq!(removed, 2);
        assert!(!db.songs.contains_key(&1));
        assert!(db.songs.contains_key(&2));
        assert!(!db.songs.contains_key(&3));
    }

    #[test]
    fn remove_source_returns_zero_when_no_match() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry { pv_id: 1, name: "A".into(), source: "base".into(), ..Default::default() });
        assert_eq!(db.remove_source("mod"), 0);
        assert_eq!(db.songs.len(), 1);
    }

    #[test]
    fn remove_unnamed_removes_entries_with_no_name_or_name_en() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry { pv_id: 1, name: "Has".into(), name_en: "HasEN".into(), ..Default::default() });
        db.songs.insert(2, SongEntry { pv_id: 2, name: "".into(), name_en: "OnlyEN".into(), ..Default::default() });
        db.songs.insert(3, SongEntry { pv_id: 3, name: "".into(), name_en: "".into(), ..Default::default() });
        let removed = db.remove_unnamed();
        assert_eq!(removed, 1);
        assert!(db.songs.contains_key(&1));
        assert!(db.songs.contains_key(&2));
        assert!(!db.songs.contains_key(&3));
    }

    // ----------------- source_stats -----------------

    #[test]
    fn source_stats_collapses_mod_prefix_to_single_bucket() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry { pv_id: 1, name: "A".into(), source: "mod:foo".into(), ..Default::default() });
        db.songs.insert(2, SongEntry { pv_id: 2, name: "B".into(), source: "mod:bar".into(), ..Default::default() });
        db.songs.insert(3, SongEntry { pv_id: 3, name: "C".into(), source: "base".into(), ..Default::default() });
        let stats = db.source_stats();
        assert_eq!(stats.get("mod"), Some(&2));
        assert_eq!(stats.get("base"), Some(&1));
        assert!(!stats.contains_key("mod:foo"));
    }

    #[test]
    fn source_stats_empty_database() {
        let db = SongDatabase::default();
        assert!(db.source_stats().is_empty());
    }

    // ----------------- apply_chinese_names -----------------

    #[test]
    fn apply_chinese_names_sets_name_zh_on_match() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry { pv_id: 1, name: "Senbonzakura".into(), ..Default::default() });
        let mut zh = HashMap::new();
        zh.insert("Senbonzakura".to_string(), "千本桜".to_string());
        db.apply_chinese_names(&zh);
        assert_eq!(db.songs.get(&1).unwrap().name_zh, Some("千本桜".to_string()));
    }

    #[test]
    fn apply_chinese_names_does_not_clear_on_miss() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry {
            pv_id: 1,
            name: "Song".to_string(),
            name_zh: Some("已有中文".to_string()),
            ..Default::default()
        });
        let zh = HashMap::new(); // 无匹配
        db.apply_chinese_names(&zh);
        assert_eq!(db.songs.get(&1).unwrap().name_zh, Some("已有中文".to_string()));
    }

    #[test]
    fn apply_chinese_names_no_op_on_empty_db() {
        let mut db = SongDatabase::default();
        let mut zh = HashMap::new();
        zh.insert("x".to_string(), "y".to_string());
        db.apply_chinese_names(&zh);
        assert!(db.songs.is_empty());
    }

    // ----------------- to_song_infos -----------------

    #[test]
    fn to_song_infos_sorts_by_pv_id() {
        let mut db = SongDatabase::default();
        db.songs.insert(30, SongEntry { pv_id: 30, name: "C".into(), ..Default::default() });
        db.songs.insert(10, SongEntry { pv_id: 10, name: "A".into(), ..Default::default() });
        db.songs.insert(20, SongEntry { pv_id: 20, name: "B".into(), ..Default::default() });
        let infos = db.to_song_infos();
        assert_eq!(infos.len(), 3);
        assert_eq!(infos[0].pv_id, 10);
        assert_eq!(infos[1].pv_id, 20);
        assert_eq!(infos[2].pv_id, 30);
    }

    #[test]
    fn to_song_infos_converts_empty_strings_to_none() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry {
            pv_id: 1,
            name: "HasName".into(),
            name_en: "".into(),
            source: "  ".into(),
            ..Default::default()
        });
        let infos = db.to_song_infos();
        assert_eq!(infos[0].name, "HasName");
        assert_eq!(infos[0].name_en, None);
        assert_eq!(infos[0].source, None);
    }

    #[test]
    fn to_song_infos_preserves_name_zh() {
        let mut db = SongDatabase::default();
        db.songs.insert(1, SongEntry {
            pv_id: 1,
            name: "X".into(),
            name_zh: Some("中文".to_string()),
            ..Default::default()
        });
        let infos = db.to_song_infos();
        assert_eq!(infos[0].name_zh, Some("中文".to_string()));
    }

    #[test]
    fn resolve_mod_names_uses_metadata_priority_and_refreshes_existing_songs() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "diva_mod_names_{}_{}",
            std::process::id(),
            unique
        ));
        let mod_dir = root.join("曲包文件夹");
        let other_dir = root.join("另一目录");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::create_dir_all(&other_dir).unwrap();
        fs::write(mod_dir.join("mod.json"), r#"{"name":"  JSON 曲包  "}"#).unwrap();
        fs::write(
            mod_dir.join("config.toml"),
            r#"name = "  中文 曲包 \"特别版\" \u66F2  "
[details]
name = "不是顶层名称"
"#,
        )
        .unwrap();
        fs::write(other_dir.join("config.toml"), "name = '另一首 曲包'").unwrap();

        let mut database = SongDatabase::default();
        for (index, source) in [
            "mod:曲包文件夹",
            "mod:曲包文件夹",
            "mod:另一目录",
            "mod:缺失目录",
            "base",
            "dlc",
            "modded",
            "",
        ]
        .into_iter()
        .enumerate()
        {
            let pv_id = index as u32;
            database.songs.insert(pv_id, SongEntry {
                pv_id,
                source: source.to_string(),
                ..Default::default()
            });
        }
        let mut songs = database.to_song_infos();
        resolve_mod_names(&mut songs, Some(&root));
        assert_eq!(
            songs.iter().map(|song| song.mod_name.as_deref()).collect::<Vec<_>>(),
            vec![
                Some("JSON 曲包"),
                Some("JSON 曲包"),
                Some("另一首 曲包"),
                Some("缺失目录"),
                None,
                None,
                None,
                None,
            ]
        );

        // 无效、空白、非字符串及仅嵌套的 JSON 名称均应继续读取 TOML。
        for json in ["{损坏", r#"{"name":" \t\n "}"#, r#"{"name":42}"#, r#"{"details":{"name":"嵌套名称"}}"#] {
            fs::write(mod_dir.join("mod.json"), json).unwrap();
            resolve_mod_names(&mut songs, Some(&root));
            assert_eq!(songs[0].mod_name.as_deref(), Some("中文 曲包 \"特别版\" 曲"));
            assert_eq!(songs[1].mod_name, songs[0].mod_name);
        }
        fs::remove_file(mod_dir.join("mod.json")).unwrap();
        resolve_mod_names(&mut songs, Some(&root));
        assert_eq!(songs[0].mod_name.as_deref(), Some("中文 曲包 \"特别版\" 曲"));

        // TOML 也不可用时回退到来源文件夹，不保留上一次请求的友好名称。
        for toml in ["name = ", "name = '   '", "[details]\nname = '嵌套名称'"] {
            fs::write(mod_dir.join("config.toml"), toml).unwrap();
            resolve_mod_names(&mut songs, Some(&root));
            assert_eq!(songs[0].mod_name.as_deref(), Some("曲包文件夹"));
        }
        fs::remove_file(mod_dir.join("config.toml")).unwrap();
        resolve_mod_names(&mut songs, Some(&root));
        assert_eq!(songs[0].mod_name.as_deref(), Some("曲包文件夹"));

        resolve_mod_names(&mut songs, None);
        assert_eq!(songs[2].mod_name.as_deref(), Some("另一目录"));
        fs::remove_dir_all(&root).unwrap();
        resolve_mod_names(&mut songs, Some(&root));
        assert_eq!(songs[2].mod_name.as_deref(), Some("另一目录"));
    }

    // ----------------- non_empty -----------------

    #[test]
    fn non_empty_returns_none_for_whitespace_only() {
        assert_eq!(non_empty(""), None);
        assert_eq!(non_empty("   "), None);
        assert_eq!(non_empty("\t\n"), None);
    }

    #[test]
    fn non_empty_returns_some_for_non_empty() {
        assert_eq!(non_empty("x"), Some("x".to_string()));
        assert_eq!(non_empty("  x  "), Some("  x  ".to_string())); // 不 trim 内容
    }
}
