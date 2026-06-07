"""
歌曲搜索模块
"""

import json
from pathlib import Path
from typing import List, Tuple, Optional, Dict

# 难度名规范化映射（用户输入关键词 → 内部 key）
DIFFICULTY_KEY_ALIASES: dict[str, str] = {
    "easy": "easy", "e": "easy",
    "normal": "normal", "n": "normal",
    "hard": "hard", "h": "hard",
    "extreme": "extreme", "ex": "extreme", "extr": "extreme",
    "exextreme": "exextreme", "exex": "exextreme", "ex2": "exextreme", "encore": "exextreme",
}

# 从 PV_LV_XX_Y 解析浮点星级
def _parse_level(level_str: str) -> float:
    try:
        parts = level_str.split("_")
        return float(f"{int(parts[2])}.{int(parts[3])}")
    except (IndexError, ValueError):
        return 0.0


class SongSearcher:
    """歌曲搜索器"""

    def __init__(self, data_dir: str = "Data", mods_dir: str = "", proxy: str = ""):
        self.data_dir = Path(data_dir)
        self.mods_dir = Path(mods_dir) if mods_dir else None
        self.proxy = proxy

        # 名称索引
        self.id_to_name: Dict[int, str] = {}        # ID -> 日文名
        self.id_to_name_en: Dict[int, str] = {}     # ID -> 英文名
        self.id_to_name_zh: Dict[int, str] = {}     # ID -> 中文名

        self.name_to_ids: Dict[str, List[int]] = {}
        self.name_to_ids_en: Dict[str, List[int]] = {}
        self.name_to_ids_zh: Dict[str, List[int]] = {}

        # 作者索引：ID -> 作者名（songinfo.music）
        self.id_to_author: Dict[int, str] = {}
        # 作者名（小写） -> [ID列表]
        self.author_to_ids: Dict[str, List[int]] = {}

        # 难度索引：ID -> {难度key -> 星级浮点}
        # 难度key: "easy" / "normal" / "hard" / "extreme" / "exextreme"
        self.id_to_difficulty: Dict[int, Dict[str, float]] = {}

        # 别名
        self.another_names: Dict[str, str] = {}
        self.alias_to_ids: Dict[str, List[int]] = {}

        # 汉字转换表
        self.hanzi_to_kanji: Dict[str, str] = {}
        self._load_hanzi_kanji_dict()

        self.translator = None

    # ------------------------------------------------------------------ #
    #  初始化                                                               #
    # ------------------------------------------------------------------ #

    def _load_hanzi_kanji_dict(self):
        hanzi_kanji_file = self.data_dir / "HanziKanjiDict.txt"
        if not hanzi_kanji_file.exists():
            print(f"警告: 汉字转换表不存在: {hanzi_kanji_file}")
            return
        with open(hanzi_kanji_file, "r", encoding="UTF-8") as f:
            hanzi_list, kanji_list = [], []
            for text in f.readlines():
                parts = text.split()
                if len(parts) >= 2:
                    hanzi_list.append(parts[0])
                    kanji_list.append(parts[1])
        self.hanzi_to_kanji = dict(zip(hanzi_list, kanji_list))

    def load_database(self):
        """加载歌曲数据库，优先读取 update_db 生成的 song_db.json。"""
        print("加载歌曲数据库...")

        structured_db = self.data_dir / "song_db.json"
        if structured_db.exists():
            if self._load_structured_database(structured_db):
                self._rebuild_reverse_indexes()
                print(f"已加载结构化数据库: {structured_db}")
                print(f"数据库加载完成，共 {len(self.id_to_name)} 首歌曲")
                if self.id_to_name_zh:
                    print(f"其中 {len(self.id_to_name_zh)} 首有中文名")
                if self.id_to_author:
                    print(f"其中 {len(self.id_to_author)} 首有作者信息")
                return

            print("警告: song_db.json 加载失败，将回退到扫描原始数据库")

        pv_db_file = self.data_dir / "pv_db.txt"
        if pv_db_file.exists():
            self._load_pvdb(pv_db_file, source="base")
            print(f"已加载主数据库: {len(self.id_to_name)} 首歌曲")

        if self.mods_dir and self.mods_dir.exists():
            self._load_mods_database()

        self._load_another_names()
        self._load_chinese_name_cache()
        self._rebuild_reverse_indexes()

        print(f"数据库加载完成，共 {len(self.id_to_name)} 首歌曲")
        if self.id_to_name_zh:
            print(f"其中 {len(self.id_to_name_zh)} 首有中文名")
        if self.id_to_author:
            print(f"其中 {len(self.id_to_author)} 首有作者信息")

    def _load_structured_database(self, db_path: Path) -> bool:
        """从 Data/song_db.json 加载结构化数据库。"""
        try:
            from diva_live_helper.song_db import ChineseNameDatabase, SongDatabase

            db = SongDatabase(db_path)
            if not db.load():
                return False

            zh_db = ChineseNameDatabase(self.data_dir / "song_name_zh.json")
            if zh_db.load():
                zh_db.apply_to_song_db(db)

            self.id_to_name.clear()
            self.id_to_name_en.clear()
            self.id_to_name_zh.clear()
            self.id_to_author.clear()
            self.id_to_difficulty.clear()
            self.another_names.clear()
            self.alias_to_ids.clear()

            for pv_id, entry in db.songs.items():
                if entry.name:
                    self.id_to_name[pv_id] = entry.name
                if entry.name_en:
                    self.id_to_name_en[pv_id] = entry.name_en
                if entry.name_zh:
                    self.id_to_name_zh[pv_id] = entry.name_zh
                if entry.authors:
                    self.id_to_author[pv_id] = entry.authors[0]
                if entry.difficulty:
                    self.id_to_difficulty[pv_id] = entry.difficulty

                for alias in entry.aliases:
                    self.alias_to_ids.setdefault(alias.lower(), []).append(pv_id)
                    # 兼容旧搜索逻辑：alias -> 原名
                    self.another_names[alias] = entry.name or entry.name_en

            return True
        except Exception as e:
            print(f"加载结构化数据库失败: {e}")
            return False

    # ------------------------------------------------------------------ #
    #  pv_db 解析                                                           #
    # ------------------------------------------------------------------ #

    def _load_pvdb(self, file_path: Path, source: str = "base"):
        """解析单个 pv_db 文件，提取名称/作者/难度"""
        lines = file_path.read_text(encoding="UTF-8").splitlines()

        # 按 pv_id 归组所有字段
        raw: Dict[int, Dict[str, str]] = {}
        for line in lines:
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line or "." not in line:
                continue
            dot = line.index(".")
            eq = line.index("=")
            pv_part = line[:dot]
            key_part = line[dot + 1:eq]
            val_part = line[eq + 1:]
            if not pv_part.startswith("pv_"):
                continue
            id_str = pv_part[3:]
            if not id_str.isdigit():
                continue
            pv_id = int(id_str)
            raw.setdefault(pv_id, {})[key_part] = val_part

        for pv_id, fields in raw.items():
            name = fields.get("song_name", "")
            name_en = fields.get("song_name_en", "")

            # 只有在有 name 的情况下才写入（MOD 可能只覆盖部分字段）
            if name:
                self.id_to_name[pv_id] = name
            if name_en:
                self.id_to_name_en[pv_id] = name_en

            # 作者
            music = (
                fields.get("songinfo.music", "")
                or fields.get("songinfo_en.music", "")
            ).strip()
            if music:
                self.id_to_author[pv_id] = music

            # 难度
            difficulty = self._extract_difficulty(fields)
            if difficulty:
                self.id_to_difficulty[pv_id] = difficulty

    def _extract_difficulty(self, fields: Dict[str, str]) -> Dict[str, float]:
        """从字段字典提取难度星级"""
        result: Dict[str, float] = {}
        for db_key, our_key in [
            ("easy", "easy"),
            ("normal", "normal"),
            ("hard", "hard"),
            ("extreme", "extreme"),
        ]:
            level = fields.get(f"difficulty.{db_key}.0.level", "")
            if level:
                result[our_key] = _parse_level(level)
        # exextreme = extreme edition 1
        exex = fields.get("difficulty.extreme.1.level", "")
        if exex:
            result["exextreme"] = _parse_level(exex)
        return result

    def _load_mods_database(self):
        mods_dir = self.mods_dir
        if mods_dir is None:
            return

        print(f"扫描MOD文件夹: {mods_dir}")
        mod_count = 0
        before = len(self.id_to_name)
        for mod_folder in mods_dir.iterdir():
            if not mod_folder.is_dir():
                continue
            mod_pv_db = mod_folder / "rom" / "mod_pv_db.txt"
            if not mod_pv_db.exists():
                continue
            try:
                self._load_pvdb(mod_pv_db, source=f"mod:{mod_folder.name}")
                mod_count += 1
                print(f"  已加载MOD: {mod_folder.name}")
            except Exception as e:
                print(f"  加载MOD失败 {mod_folder.name}: {e}")
        print(f"已加载 {mod_count} 个MOD，新增 {len(self.id_to_name) - before} 首歌曲")

    def _load_another_names(self):
        another_name_file = self.data_dir / "AnotherSongName.json"
        if not another_name_file.exists():
            print(f"警告: 别名数据库不存在: {another_name_file}")
            return
        with open(another_name_file, "r", encoding="UTF-8") as f:
            self.another_names = json.load(f)
        print(f"已加载 {len(self.another_names)} 个别名")

    def _load_chinese_name_cache(self):
        zh_db_file = self.data_dir / "song_name_zh.json"
        if zh_db_file.exists():
            try:
                with open(zh_db_file, "r", encoding="utf-8") as f:
                    raw = json.load(f)
                raw_names = raw.get("names", raw) if isinstance(raw, dict) else {}
                if isinstance(raw_names, dict):
                    # 格式：原曲名 -> 中文名；构建反向索引用于填入 id_to_name_zh
                    name_to_id: dict[str, int] = {
                        name: pv_id for pv_id, name in self.id_to_name.items()
                    }
                    for song_name, zh_name in raw_names.items():
                        if isinstance(song_name, str) and isinstance(zh_name, str) and zh_name:
                            pv_id = name_to_id.get(song_name)
                            if pv_id is not None:
                                self.id_to_name_zh[pv_id] = zh_name
                print(f"已加载 {len(self.id_to_name_zh)} 条中文名缓存")
                return
            except Exception as e:
                print(f"加载中文名数据库失败: {e}")

        cache_file = self.data_dir / "song_name_cache.json"
        if not cache_file.exists():
            return
        try:
            with open(cache_file, "r", encoding="utf-8") as f:
                cache: Dict[str, str] = json.load(f)
            for pv_id, ja_name in self.id_to_name.items():
                if ja_name in cache:
                    self.id_to_name_zh[pv_id] = cache[ja_name]
            print(f"已加载 {len(self.id_to_name_zh)} 条中文名缓存")
        except Exception as e:
            print(f"加载中文名缓存失败: {e}")

    def _rebuild_reverse_indexes(self):
        """重建所有反向索引（名称 / 作者）"""
        self.name_to_ids = {}
        for pv_id, name in self.id_to_name.items():
            self.name_to_ids.setdefault(name, []).append(pv_id)

        self.name_to_ids_en = {}
        for pv_id, name in self.id_to_name_en.items():
            self.name_to_ids_en.setdefault(name, []).append(pv_id)

        self.name_to_ids_zh = {}
        for pv_id, name in self.id_to_name_zh.items():
            self.name_to_ids_zh.setdefault(name, []).append(pv_id)

        self.author_to_ids = {}
        for pv_id, author in self.id_to_author.items():
            self.author_to_ids.setdefault(author.lower(), []).append(pv_id)

        # 旧版 AnotherSongName.json: alias -> real_name
        # 结构化 song_db.json 加载时也会填充 another_names，用这里统一生成 alias_to_ids。
        self.alias_to_ids = {}
        name_index: Dict[str, List[int]] = {}
        for name, ids in self.name_to_ids.items():
            name_index.setdefault(name.lower(), []).extend(ids)
        for name, ids in self.name_to_ids_en.items():
            name_index.setdefault(name.lower(), []).extend(ids)
        for name, ids in self.name_to_ids_zh.items():
            name_index.setdefault(name.lower(), []).extend(ids)

        for alias, real_name in self.another_names.items():
            ids = name_index.get(real_name.lower(), [])
            if ids:
                self.alias_to_ids.setdefault(alias.lower(), []).extend(ids)

    # ------------------------------------------------------------------ #
    #  搜索                                                                 #
    # ------------------------------------------------------------------ #

    def search(
        self,
        query: str,
        difficulty: Optional[float] = None,
        difficulty_key: str = "extreme",
        difficulty_tolerance: float = 0.5,
    ) -> List[Tuple[int, str]]:
        """
        搜索歌曲（按名称/别名）。

        Args:
            query: 搜索关键词
            difficulty: 目标难度星级（如 7.0）；None = 不过滤
            difficulty_key: 用哪个难度档比较，默认 "extreme"
            difficulty_tolerance: 允许的星级误差范围（±），默认 ±0.5

        Returns:
            [(pv_id, display_name), ...]，按相关性排序
        """
        candidates: List[Tuple[int, str]] = []

        # 1. 日文名精确/模糊
        for name, ids in self.name_to_ids.items():
            if query.lower() in name.lower():
                for pv_id in ids:
                    candidates.append((pv_id, name))

        # 2. 中文名
        for name, ids in self.name_to_ids_zh.items():
            if query.lower() in name.lower():
                for pv_id in ids:
                    entry = (pv_id, name)
                    if entry not in candidates:
                        candidates.append(entry)

        # 3. 汉字转假名
        if not candidates:
            converted = self._hanzi_to_kanji_convert(query)
            if converted != query:
                for name, ids in self.name_to_ids.items():
                    if converted.lower() in name.lower():
                        for pv_id in ids:
                            candidates.append((pv_id, name))

        # 4. 别名
        if not candidates:
            for alias, ids in self.alias_to_ids.items():
                if query.lower() in alias:
                    for pv_id in ids:
                        candidates.append((pv_id, self.get_display_name(pv_id)))

        if not candidates:
            for alias, real_name in self.another_names.items():
                if query.lower() in alias.lower():
                    for name, ids in self.name_to_ids.items():
                        if real_name.lower() == name.lower():
                            for pv_id in ids:
                                candidates.append((pv_id, name))

        # 5. 英文名
        for name, ids in self.name_to_ids_en.items():
            if query.lower() in name.lower():
                for pv_id in ids:
                    entry = (pv_id, name)
                    if entry not in candidates:
                        candidates.append(entry)

        # 难度过滤
        if difficulty is not None and candidates:
            candidates = self._filter_by_difficulty(
                candidates, difficulty, difficulty_key, difficulty_tolerance
            )

        return candidates

    def search_by_author(
        self,
        author: str,
        difficulty: Optional[float] = None,
        difficulty_key: str = "extreme",
        difficulty_tolerance: float = 0.5,
    ) -> List[Tuple[int, str]]:
        """
        按作者名搜索（匹配 songinfo.music）。

        Args:
            author: 作者名关键词（模糊匹配）
            difficulty: 同 search()
            difficulty_key: 同 search()
            difficulty_tolerance: 同 search()

        Returns:
            [(pv_id, display_name), ...]
        """
        results: List[Tuple[int, str]] = []
        author_lower = author.lower()

        for stored_author, ids in self.author_to_ids.items():
            if author_lower in stored_author:
                for pv_id in ids:
                    display = (
                        self.id_to_name_zh.get(pv_id)
                        or self.id_to_name.get(pv_id)
                        or self.id_to_name_en.get(pv_id)
                        or f"Unknown({pv_id})"
                    )
                    entry = (pv_id, display)
                    if entry not in results:
                        results.append(entry)

        if difficulty is not None and results:
            results = self._filter_by_difficulty(
                results, difficulty, difficulty_key, difficulty_tolerance
            )

        return results

    def _filter_by_difficulty(
        self,
        candidates: List[Tuple[int, str]],
        target: float,
        diff_key: str,
        tolerance: float,
    ) -> List[Tuple[int, str]]:
        """
        按难度星级过滤候选列表。

        diff_key 不存在时跳过该曲（不过滤掉，因为 MOD 曲可能没有难度信息）。
        """
        filtered = []
        for pv_id, name in candidates:
            diffs = self.id_to_difficulty.get(pv_id)
            if diffs is None:
                # 没有难度信息（MOD曲等）保留
                filtered.append((pv_id, name))
                continue
            level = diffs.get(diff_key)
            if level is None:
                # 该难度档不存在（如没有 exextreme）保留，让上层决定
                filtered.append((pv_id, name))
                continue
            if abs(level - target) <= tolerance:
                filtered.append((pv_id, name))
        return filtered

    # ------------------------------------------------------------------ #
    #  辅助                                                                 #
    # ------------------------------------------------------------------ #

    def _hanzi_to_kanji_convert(self, query: str) -> str:
        if not self.hanzi_to_kanji:
            return query
        return query.translate(str.maketrans(self.hanzi_to_kanji))

    def get_song_name(self, pv_id: int) -> Optional[str]:
        return self.id_to_name.get(pv_id)

    def get_song_name_en(self, pv_id: int) -> Optional[str]:
        return self.id_to_name_en.get(pv_id)

    def get_song_name_zh(self, pv_id: int) -> Optional[str]:
        return self.id_to_name_zh.get(pv_id)

    def get_display_name(self, pv_id: int) -> str:
        return (
            self.id_to_name_zh.get(pv_id)
            or self.id_to_name.get(pv_id)
            or self.id_to_name_en.get(pv_id)
            or f"Unknown({pv_id})"
        )

    def get_author(self, pv_id: int) -> Optional[str]:
        return self.id_to_author.get(pv_id)

    def get_difficulty(self, pv_id: int) -> Optional[Dict[str, float]]:
        return self.id_to_difficulty.get(pv_id)

    def check_id(self, pv_id: int) -> bool:
        return pv_id in self.id_to_name
