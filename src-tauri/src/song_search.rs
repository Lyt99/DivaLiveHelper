use std::collections::HashMap;

use serde::Serialize;

use crate::song_db::SongDatabase;

const EMBEDDED_HANZI_KANJI_DICT: &str = include_str!("../../Data/HanziKanjiDict.txt");

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub pv_id: u32,
    pub display_name: String,
    pub difficulty: Option<f32>,
    /// 实际匹配到的难度档位名（当首选档位缺失时，由 `difficulty_for` fallback 决定）。
    /// 为 `None` 表示歌曲没有任何难度数据。
    pub difficulty_tier: Option<String>,
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
    pub fn from_database(database: &SongDatabase) -> Self {
        let mut searcher = Self::default();
        searcher.hanzi_to_kanji = parse_hanzi_kanji(EMBEDDED_HANZI_KANJI_DICT);

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
        difficulty_fallback: &str,
        tolerance: f32,
    ) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let query_normalized = normalize_search_text(query);
        let mut candidates = Vec::new();

        self.collect_matches(
            &self.name_to_ids,
            &query_lower,
            &query_normalized,
            &mut candidates,
            false,
        );
        self.collect_matches(
            &self.name_to_ids_zh,
            &query_lower,
            &query_normalized,
            &mut candidates,
            false,
        );

        if candidates.is_empty() {
            let converted = self.hanzi_to_kanji_convert(query);
            if converted != query {
                self.collect_matches(
                    &self.name_to_ids,
                    &converted.to_lowercase(),
                    &normalize_search_text(&converted),
                    &mut candidates,
                    false,
                );
            }
        }

        if candidates.is_empty() {
            self.collect_matches(
                &self.alias_to_ids,
                &query_lower,
                &query_normalized,
                &mut candidates,
                true,
            );
        }

        self.collect_matches(
            &self.name_to_ids_en,
            &query_lower,
            &query_normalized,
            &mut candidates,
            false,
        );
        dedupe(&mut candidates);

        if let Some(target) = difficulty {
            candidates = self.filter_by_difficulty(candidates, target, difficulty_key, tolerance);
        }

        candidates
            .into_iter()
            .map(|(pv_id, display_name)| {
                let (difficulty, difficulty_tier) = self
                    .difficulty_for(pv_id, difficulty_key, difficulty_fallback)
                    .map_or((None, None), |(level, tier)| (Some(level), Some(tier)));
                SearchResult {
                    pv_id,
                    display_name,
                    difficulty,
                    difficulty_tier,
                }
            })
            .collect()
    }

    pub fn search_by_author(
        &self,
        author: &str,
        difficulty: Option<f32>,
        difficulty_key: &str,
        difficulty_fallback: &str,
        tolerance: f32,
    ) -> Vec<SearchResult> {
        let author_lower = author.to_lowercase();
        let author_normalized = normalize_search_text(author);
        let mut candidates = Vec::new();
        for (stored_author, ids) in &self.author_to_ids {
            if stored_author.contains(&author_lower)
                || (!author_normalized.is_empty()
                    && normalize_search_text(stored_author).contains(&author_normalized))
            {
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
            .map(|(pv_id, display_name)| {
                let (difficulty, difficulty_tier) = self
                    .difficulty_for(pv_id, difficulty_key, difficulty_fallback)
                    .map_or((None, None), |(level, tier)| (Some(level), Some(tier)));
                SearchResult {
                    pv_id,
                    display_name,
                    difficulty,
                    difficulty_tier,
                }
            })
            .collect()
    }

    pub fn check_id(&self, pv_id: u32) -> bool {
        self.id_to_name.contains_key(&pv_id)
    }

    const DIFFICULTY_TIERS: [&str; 5] = ["easy", "normal", "hard", "extreme", "exextreme"];

    fn difficulty_for(
        &self,
        pv_id: u32,
        difficulty_key: &str,
        fallback: &str,
    ) -> Option<(f32, String)> {
        let Some(difficulties) = self.id_to_difficulty.get(&pv_id) else {
            return None;
        };
        if let Some(level) = difficulties.get(difficulty_key).copied() {
            return Some((level, difficulty_key.to_string()));
        }
        // Preferred tier not found — try adjacent tiers based on fallback direction
        let Some(start) = Self::DIFFICULTY_TIERS
            .iter()
            .position(|&t| t == difficulty_key)
        else {
            return None;
        };
        let tiers: Vec<usize> = if fallback == "harder" {
            // Try harder first, then easier
            let mut order: Vec<usize> = (start + 1..Self::DIFFICULTY_TIERS.len()).collect();
            order.extend((0..start).rev());
            order
        } else {
            // Default: try easier first, then harder
            let mut order: Vec<usize> = (0..start).rev().collect();
            order.extend(start + 1..Self::DIFFICULTY_TIERS.len());
            order
        };
        for idx in tiers {
            let tier = Self::DIFFICULTY_TIERS[idx];
            if let Some(level) = difficulties.get(tier).copied() {
                return Some((level, tier.to_string()));
            }
        }
        None
    }

    fn collect_matches(
        &self,
        index: &HashMap<String, Vec<u32>>,
        query_lower: &str,
        query_normalized: &str,
        candidates: &mut Vec<(u32, String)>,
        use_display_name: bool,
    ) {
        for (name, ids) in index {
            let name_lower = name.to_lowercase();
            let normalized_match = !query_normalized.is_empty()
                && normalize_search_text(name).contains(query_normalized);
            if name_lower.contains(query_lower) || normalized_match {
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

fn parse_hanzi_kanji(content: &str) -> HashMap<char, char> {
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

fn normalize_search_text(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个仅包含指定歌曲与难度映射的 `SongSearcher`，避免磁盘 I/O。
    /// `difficulties` 为 `(tier, stars)` 列表，例如 `[("extreme", 9.5)]`。
    fn make_searcher(pv_id: u32, name: &str, difficulties: &[(&str, f32)]) -> SongSearcher {
        let mut searcher = SongSearcher::default();
        searcher.id_to_name.insert(pv_id, name.to_string());
        searcher.name_to_ids.insert(name.to_string(), vec![pv_id]);
        if !difficulties.is_empty() {
            let mut map = HashMap::new();
            for (tier, stars) in difficulties {
                map.insert(tier.to_string(), *stars);
            }
            searcher.id_to_difficulty.insert(pv_id, map);
        }
        searcher
    }

    // ----------------- difficulty_for -----------------

    #[test]
    fn difficulty_for_exact_tier_match_returns_that_tier() {
        // 歌曲拥有 exextreme 难度，请求 exextreme → 应直接返回 exextreme 的星级与档位名
        let searcher = make_searcher(1, "Song A", &[("extreme", 8.0), ("exextreme", 9.5)]);
        let result = searcher.difficulty_for(1, "exextreme", "easier");
        assert_eq!(result, Some((9.5, "exextreme".to_string())));
    }

    #[test]
    fn difficulty_for_easier_fallback_walks_down_first() {
        // 请求 exextreme 但歌曲只有 extreme+hard，fallback="easier" 应先试 extreme
        let searcher = make_searcher(2, "Song B", &[("hard", 5.0), ("extreme", 8.0)]);
        let result = searcher.difficulty_for(2, "exextreme", "easier");
        assert_eq!(result, Some((8.0, "extreme".to_string())));
    }

    #[test]
    fn difficulty_for_harder_fallback_walks_up_first() {
        // 请求 hard 但歌曲只有 extreme+exextreme，fallback="harder" 应先试 extreme
        let searcher = make_searcher(3, "Song C", &[("extreme", 8.0), ("exextreme", 9.5)]);
        let result = searcher.difficulty_for(3, "hard", "harder");
        assert_eq!(result, Some((8.0, "extreme".to_string())));
    }

    #[test]
    fn difficulty_for_easier_fallback_wraps_around_to_harder() {
        // 请求 extreme 但歌曲只有 exextreme，fallback="easier" 试完 [hard,normal,easy] 后回绕到 exextreme
        let searcher = make_searcher(4, "Song D", &[("exextreme", 9.5)]);
        let result = searcher.difficulty_for(4, "extreme", "easier");
        assert_eq!(result, Some((9.5, "exextreme".to_string())));
    }

    #[test]
    fn difficulty_for_returns_none_when_song_has_no_difficulty_data() {
        let searcher = make_searcher(5, "Song E", &[]);
        let result = searcher.difficulty_for(5, "extreme", "easier");
        assert_eq!(result, None);
    }

    #[test]
    fn difficulty_for_returns_none_when_all_tiers_missing() {
        // 歌曲有难度数据但请求的档位与 fallback 遍历都未命中（这里不可能完全遍历空，
        // 因为 fallback 会遍历所有 5 个档位；只要任意一档存在就会命中）。
        // 此用例验证：难度 map 为空 HashMap 时返回 None。
        let mut searcher = SongSearcher::default();
        searcher.id_to_name.insert(6, "Song F".to_string());
        searcher.name_to_ids.insert("Song F".to_string(), vec![6]);
        searcher.id_to_difficulty.insert(6, HashMap::new());
        let result = searcher.difficulty_for(6, "extreme", "easier");
        assert_eq!(result, None);
    }

    #[test]
    fn difficulty_for_returns_none_for_unknown_tier_key() {
        // 未知档位名（不在 DIFFICULTY_TIERS 中）且歌曲也没有该键 → None
        let searcher = make_searcher(7, "Song G", &[("extreme", 8.0)]);
        let result = searcher.difficulty_for(7, "master", "easier");
        assert_eq!(result, None);
    }

    // ----------------- search end-to-end (bug regression) -----------------

    #[test]
    fn search_propagates_fallback_tier_name_into_search_result() {
        // 回归测试：default_search_difficulty = "exextreme"，但歌曲只有 extreme 难度。
        // 修复前：SearchResult.difficulty_tier 错误地使用 config 默认值 "exextreme"
        //         → 切歌写入 DIFFICULTY_SELECT=4（exextreme 标签页）但歌曲无该难度
        // 修复后：difficulty_for fallback 到 extreme，SearchResult.difficulty_tier = "extreme"
        let searcher = make_searcher(10, "千本桜", &[("extreme", 8.5)]);

        // difficulty=None → 过滤器旁路（与生产弹幕路径一致）
        let results = searcher.search("千本桜", None, "exextreme", "easier", 0.5);

        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.pv_id, 10);
        assert_eq!(result.difficulty, Some(8.5));
        assert_eq!(result.difficulty_tier, Some("extreme".to_string()));
    }

    #[test]
    fn search_returns_none_tier_when_song_has_no_difficulty_data() {
        // 歌曲无任何难度数据 → difficulty 与 difficulty_tier 均为 None
        let searcher = make_searcher(11, "NoDiff Song", &[]);
        let results = searcher.search("nodiff", None, "extreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].difficulty, None);
        assert_eq!(results[0].difficulty_tier, None);
    }

    #[test]
    fn search_is_case_insensitive_for_mixed_case_titles() {
        // 用户输入不应必须和曲库标题大小写完全一致。
        let searcher = make_searcher(12, "Fire◎Flower", &[("extreme", 8.0)]);
        let results = searcher.search("fire◎flower", None, "extreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pv_id, 12);
    }

    #[test]
    fn search_ignores_symbols_and_spaces_in_titles() {
        // 回归测试：Fire◎Flower 中的 ◎ 难以输入，用户输入 fire flower 也应命中。
        let searcher = make_searcher(13, "Fire◎Flower", &[("extreme", 8.0)]);
        let results = searcher.search("fire flower", None, "extreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pv_id, 13);
    }

    #[test]
    fn search_ignores_symbols_in_english_names() {
        let mut searcher = make_searcher(14, "Japanese Title", &[("extreme", 8.0)]);
        searcher.id_to_name_en.insert(14, "Shake it!".to_string());
        searcher
            .name_to_ids_en
            .insert("Shake it!".to_string(), vec![14]);

        let results = searcher.search("shakeit", None, "extreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pv_id, 14);
    }

    #[test]
    fn search_ignores_symbols_in_aliases() {
        let mut searcher = make_searcher(15, "Fire◎Flower", &[("extreme", 8.0)]);
        searcher
            .alias_to_ids
            .insert("fire◎flower".to_string(), vec![15]);

        let results = searcher.search("fire flower", None, "extreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pv_id, 15);
    }

    #[test]
    fn search_by_author_ignores_symbols_and_case() {
        let mut searcher = SongSearcher::default();
        searcher.id_to_name.insert(16, "Author Song".to_string());
        searcher
            .id_to_author
            .insert(16, "OSTER project".to_string());
        searcher
            .author_to_ids
            .insert("oster project".to_string(), vec![16]);
        searcher
            .id_to_difficulty
            .insert(16, HashMap::from([("extreme".to_string(), 8.0)]));

        let results = searcher.search_by_author("oster-project", None, "extreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pv_id, 16);
    }

    #[test]
    fn parse_hanzi_kanji_loads_embedded_mapping() {
        let mapping = parse_hanzi_kanji(EMBEDDED_HANZI_KANJI_DICT);
        assert!(mapping.len() > 100);
        assert_eq!(mapping.get(&'爱'), Some(&'愛'));
    }

    #[test]
    fn search_matches_japanese_title_via_hanzi_to_kanji_conversion() {
        let mut searcher = SongSearcher::default();
        searcher.hanzi_to_kanji = parse_hanzi_kanji(EMBEDDED_HANZI_KANJI_DICT);
        searcher.id_to_name.insert(17, "愛言葉".to_string());
        searcher.name_to_ids.insert("愛言葉".to_string(), vec![17]);
        searcher
            .id_to_difficulty
            .insert(17, HashMap::from([("extreme".to_string(), 8.0)]));

        let results = searcher.search("爱言叶", None, "extreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pv_id, 17);
    }

    #[test]
    fn normalize_search_text_lowercases_and_keeps_cjk() {
        assert_eq!(normalize_search_text("Fire◎Flower"), "fireflower");
        assert_eq!(normalize_search_text("fire flower"), "fireflower");
        assert_eq!(normalize_search_text("FIRE-FLOWER!!"), "fireflower");
        assert_eq!(normalize_search_text("千本桜"), "千本桜");
        assert_eq!(
            normalize_search_text("みくみくにしてあげる♪"),
            "みくみくにしてあげる"
        );
        assert_eq!(normalize_search_text("!!!!"), "");
    }

    // ----------------- filter_by_difficulty -----------------

    #[test]
    fn filter_by_difficulty_keeps_songs_within_tolerance_window() {
        // target=8.0, tolerance=0.5 → [7.5, 8.5] 范围内的歌曲保留
        let mut searcher = SongSearcher::default();
        searcher
            .id_to_difficulty
            .insert(100, HashMap::from([("extreme".to_string(), 8.0)]));
        searcher
            .id_to_difficulty
            .insert(101, HashMap::from([("extreme".to_string(), 8.3)]));
        let candidates = vec![(100, "A".to_string()), (101, "B".to_string())];
        let kept = searcher.filter_by_difficulty(candidates, 8.0, "extreme", 0.5);
        let kept_ids: Vec<u32> = kept.iter().map(|(id, _)| *id).collect();
        assert_eq!(kept_ids, vec![100, 101]);
    }

    #[test]
    fn filter_by_difficulty_drops_songs_outside_tolerance() {
        // target=8.0, tolerance=0.5 → 9.5 超出 [7.5, 8.5] 被剔除
        let mut searcher = SongSearcher::default();
        searcher
            .id_to_difficulty
            .insert(100, HashMap::from([("extreme".to_string(), 8.0)]));
        searcher
            .id_to_difficulty
            .insert(101, HashMap::from([("extreme".to_string(), 9.5)]));
        let candidates = vec![(100, "A".to_string()), (101, "B".to_string())];
        let kept = searcher.filter_by_difficulty(candidates, 8.0, "extreme", 0.5);
        let kept_ids: Vec<u32> = kept.iter().map(|(id, _)| *id).collect();
        assert_eq!(kept_ids, vec![100]);
    }

    #[test]
    fn filter_by_difficulty_keeps_songs_missing_requested_tier() {
        // 歌曲没有请求的档位 → 保留（不参与过滤）
        let mut searcher = SongSearcher::default();
        searcher
            .id_to_difficulty
            .insert(100, HashMap::from([("hard".to_string(), 5.0)]));
        let candidates = vec![(100, "A".to_string())];
        let kept = searcher.filter_by_difficulty(candidates, 9.0, "exextreme", 0.5);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn filter_by_difficulty_keeps_songs_with_no_difficulty_data() {
        // 歌曲完全没有难度数据 → 保留
        let searcher = SongSearcher::default();
        let candidates = vec![(200, "Bare Song".to_string())];
        let kept = searcher.filter_by_difficulty(candidates, 9.0, "extreme", 0.5);
        assert_eq!(kept.len(), 1);
    }

    // ----------------- search_by_author tier propagation -----------------

    #[test]
    fn search_by_author_propagates_fallback_tier_name() {
        // 与 search() 同样的 fallback 行为应作用于 search_by_author
        let mut searcher = SongSearcher::default();
        searcher.id_to_name.insert(20, "Miku Song".to_string());
        searcher.id_to_author.insert(20, "ryo".to_string());
        searcher.author_to_ids.insert("ryo".to_string(), vec![20]);
        searcher
            .id_to_difficulty
            .insert(20, HashMap::from([("extreme".to_string(), 8.0)]));

        let results = searcher.search_by_author("ryo", None, "exextreme", "easier", 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].difficulty, Some(8.0));
        assert_eq!(results[0].difficulty_tier, Some("extreme".to_string()));
    }
}
