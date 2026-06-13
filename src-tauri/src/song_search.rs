use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::song_db::SongDatabase;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub pv_id: u32,
    pub display_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct SongSearcher {
    id_to_name: HashMap<u32, String>,
    id_to_name_en: HashMap<u32, String>,
    id_to_name_zh: HashMap<u32, String>,
    id_to_author: HashMap<u32, String>,
    id_to_difficulty: HashMap<u32, HashMap<String, f32>>,
    name_to_ids: HashMap<String, Vec<u32>>,
    name_to_ids_en: HashMap<String, Vec<u32>>,
    name_to_ids_zh: HashMap<String, Vec<u32>>,
    author_to_ids: HashMap<String, Vec<u32>>,
    alias_to_ids: HashMap<String, Vec<u32>>,
    hanzi_to_kanji: HashMap<char, char>,
}

impl SongSearcher {
    pub fn from_database(database: &SongDatabase, data_dir: &Path) -> Self {
        let mut searcher = Self::default();
        searcher.hanzi_to_kanji = load_hanzi_kanji(&data_dir.join("HanziKanjiDict.txt"));

        for entry in database.songs.values() {
            let pv_id = entry.pv_id;
            if !entry.name.is_empty() {
                searcher.id_to_name.insert(pv_id, entry.name.clone());
                searcher
                    .name_to_ids
                    .entry(entry.name.clone())
                    .or_default()
                    .push(pv_id);
            }
            if !entry.name_en.is_empty() {
                searcher.id_to_name_en.insert(pv_id, entry.name_en.clone());
                searcher
                    .name_to_ids_en
                    .entry(entry.name_en.clone())
                    .or_default()
                    .push(pv_id);
            }
            if let Some(name_zh) = &entry.name_zh {
                searcher.id_to_name_zh.insert(pv_id, name_zh.clone());
                searcher
                    .name_to_ids_zh
                    .entry(name_zh.clone())
                    .or_default()
                    .push(pv_id);
            }
            if let Some(author) = entry.authors.first() {
                searcher.id_to_author.insert(pv_id, author.clone());
                searcher
                    .author_to_ids
                    .entry(author.to_lowercase())
                    .or_default()
                    .push(pv_id);
            }
            if !entry.difficulty.is_empty() {
                searcher
                    .id_to_difficulty
                    .insert(pv_id, entry.difficulty.clone());
            }
            for alias in &entry.aliases {
                searcher
                    .alias_to_ids
                    .entry(alias.to_lowercase())
                    .or_default()
                    .push(pv_id);
            }
        }

        searcher
    }

    pub fn search(
        &self,
        query: &str,
        difficulty: Option<f32>,
        difficulty_key: &str,
        tolerance: f32,
    ) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut candidates = Vec::new();

        self.collect_matches(&self.name_to_ids, &query_lower, &mut candidates, false);
        self.collect_matches(&self.name_to_ids_zh, &query_lower, &mut candidates, false);

        if candidates.is_empty() {
            let converted = self.hanzi_to_kanji_convert(query);
            if converted != query {
                self.collect_matches(
                    &self.name_to_ids,
                    &converted.to_lowercase(),
                    &mut candidates,
                    false,
                );
            }
        }

        if candidates.is_empty() {
            self.collect_matches(&self.alias_to_ids, &query_lower, &mut candidates, true);
        }

        self.collect_matches(&self.name_to_ids_en, &query_lower, &mut candidates, false);
        dedupe(&mut candidates);

        if let Some(target) = difficulty {
            candidates = self.filter_by_difficulty(candidates, target, difficulty_key, tolerance);
        }

        candidates
            .into_iter()
            .map(|(pv_id, display_name)| SearchResult {
                pv_id,
                display_name,
            })
            .collect()
    }

    pub fn search_by_author(
        &self,
        author: &str,
        difficulty: Option<f32>,
        difficulty_key: &str,
        tolerance: f32,
    ) -> Vec<SearchResult> {
        let author_lower = author.to_lowercase();
        let mut candidates = Vec::new();
        for (stored_author, ids) in &self.author_to_ids {
            if stored_author.contains(&author_lower) {
                for pv_id in ids {
                    candidates.push((*pv_id, self.display_name(*pv_id)));
                }
            }
        }
        dedupe(&mut candidates);
        if let Some(target) = difficulty {
            candidates = self.filter_by_difficulty(candidates, target, difficulty_key, tolerance);
        }
        candidates
            .into_iter()
            .map(|(pv_id, display_name)| SearchResult {
                pv_id,
                display_name,
            })
            .collect()
    }

    pub fn check_id(&self, pv_id: u32) -> bool {
        self.id_to_name.contains_key(&pv_id)
    }

    fn collect_matches(
        &self,
        index: &HashMap<String, Vec<u32>>,
        query_lower: &str,
        candidates: &mut Vec<(u32, String)>,
        use_display_name: bool,
    ) {
        for (name, ids) in index {
            if name.to_lowercase().contains(query_lower) {
                for pv_id in ids {
                    let display = if use_display_name {
                        self.display_name(*pv_id)
                    } else {
                        name.clone()
                    };
                    candidates.push((*pv_id, display));
                }
            }
        }
    }

    fn filter_by_difficulty(
        &self,
        candidates: Vec<(u32, String)>,
        target: f32,
        difficulty_key: &str,
        tolerance: f32,
    ) -> Vec<(u32, String)> {
        candidates
            .into_iter()
            .filter(|(pv_id, _)| {
                let Some(difficulties) = self.id_to_difficulty.get(pv_id) else {
                    return true;
                };
                let Some(level) = difficulties.get(difficulty_key) else {
                    return true;
                };
                (*level - target).abs() <= tolerance
            })
            .collect()
    }

    fn display_name(&self, pv_id: u32) -> String {
        self.id_to_name_zh
            .get(&pv_id)
            .or_else(|| self.id_to_name.get(&pv_id))
            .or_else(|| self.id_to_name_en.get(&pv_id))
            .cloned()
            .unwrap_or_else(|| format!("Unknown({pv_id})"))
    }

    fn hanzi_to_kanji_convert(&self, query: &str) -> String {
        query
            .chars()
            .map(|character| {
                self.hanzi_to_kanji
                    .get(&character)
                    .copied()
                    .unwrap_or(character)
            })
            .collect()
    }
}

fn load_hanzi_kanji(path: &Path) -> HashMap<char, char> {
    let Ok(content) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let mut mapping = HashMap::new();
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let Some(from) = parts.next().and_then(|part| part.chars().next()) else {
            continue;
        };
        let Some(to) = parts.next().and_then(|part| part.chars().next()) else {
            continue;
        };
        mapping.insert(from, to);
    }
    mapping
}

fn dedupe(candidates: &mut Vec<(u32, String)>) {
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|(pv_id, display_name)| seen.insert((*pv_id, display_name.clone())));
}
