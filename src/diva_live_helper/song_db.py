"""
song_db.py — 结构化歌曲数据库

统一的数据模型，支持：
- 通过作者搜索
- 通过难度搜索
- 每首曲子独立管理别名
- 可分享给他人（基于 pv_id，与 mod 安装无关）

文件格式（Data/song_db.json）：
{
    "version": 1,
    "songs": {
        "1": {           # key 为 pv_id 字符串
            "pv_id": 1,
            "name": "恋は戦争",          # 日文/原文名
            "name_en": "Love is War",   # 英文名（可为空）
            "aliases": ["恋战", "lyw"], # 用户自定义别名列表
            "authors": ["ryo"],         # 来自 songinfo.music
            "difficulty": {             # 各难度星级，0.0 表示无此难度
                "easy": 2.0,
                "normal": 4.0,
                "hard": 6.0,
                "extreme": 8.0,
                "exextreme": 8.5
            },
            "source": "base"           # "base" / "mod:<mod_name>"
        }
    }
}
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path


# 难度名规范化映射（用户输入 → DB key）
DIFFICULTY_ALIASES: dict[str, str] = {
    # easy
    "easy": "easy",
    "e": "easy",
    # normal
    "normal": "normal",
    "n": "normal",
    # hard
    "hard": "hard",
    "h": "hard",
    # extreme
    "extreme": "extreme",
    "ex": "extreme",
    "extr": "extreme",
    # exextreme / extra extreme / encore
    "exextreme": "exextreme",
    "exex": "exextreme",
    "ex2": "exextreme",
    "encore": "exextreme",
}

DB_VERSION = 1


def safe_print(message: str):
    """兼容 Windows 非 UTF-8 控制台输出（如 emoji 别名）。"""
    try:
        print(message)
    except UnicodeEncodeError:
        encoding = sys.stdout.encoding or "utf-8"
        print(message.encode(encoding, errors="replace").decode(encoding, errors="replace"))


@dataclass
class SongEntry:
    """单首歌曲的结构化数据"""

    pv_id: int
    name: str                         # 日文/原文名（来自 song_name）
    name_en: str = ""                 # 英文名（来自 song_name_en）
    name_zh: str = ""                 # 中文名（运行时从 song_name_zh.json 叠加，不写入 song_db.json）
    aliases: list[str] = field(default_factory=list)
    authors: list[str] = field(default_factory=list)   # 来自 songinfo.music
    difficulty: dict[str, float] = field(default_factory=dict)
    source: str = "base"              # "base" / "mod:<name>"

    def to_dict(self) -> dict[str, object]:
        return {
            "pv_id": self.pv_id,
            "name": self.name,
            "name_en": self.name_en,
            "aliases": self.aliases,
            "authors": self.authors,
            "difficulty": self.difficulty,
            "source": self.source,
        }

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> "SongEntry":
        pv_id_raw = d.get("pv_id", 0)
        aliases_raw = d.get("aliases", [])
        authors_raw = d.get("authors", [])
        difficulty_raw = d.get("difficulty", {})

        return cls(
            pv_id=int(pv_id_raw) if isinstance(pv_id_raw, int | str) else 0,
            name=str(d.get("name", "")),
            name_en=str(d.get("name_en", "")),
            name_zh=str(d.get("name_zh", "")),
            aliases=[str(item) for item in aliases_raw] if isinstance(aliases_raw, list) else [],
            authors=[str(item) for item in authors_raw] if isinstance(authors_raw, list) else [],
            difficulty={
                str(key): float(value)
                for key, value in difficulty_raw.items()
                if isinstance(difficulty_raw, dict) and isinstance(value, int | float)
            } if isinstance(difficulty_raw, dict) else {},
            source=str(d.get("source", "base")),
        )

    def display_name(self) -> str:
        """优先中文名，其次日文名，最后英文名"""
        return self.name_zh or self.name or self.name_en or f"Unknown({self.pv_id})"


def parse_level(level_str: str) -> float:
    """
    将 PV_LV_08_5 格式转换为浮点数 8.5
    PV_LV_08_0 → 8.0
    """
    # 格式: PV_LV_{整数}_{小数位}
    try:
        parts = level_str.split("_")
        # parts = ['PV', 'LV', '08', '5'] or ['PV', 'LV', '08', '0']
        integer = int(parts[2])
        decimal = int(parts[3])
        return float(f"{integer}.{decimal}")
    except (IndexError, ValueError):
        return 0.0


class SongDatabase:
    """
    歌曲数据库 — 负责加载/保存/合并 song_db.json

    设计原则：
    - pv_id 是主键，字符串形式作为 JSON key
    - 合并时 base 数据可被 mod 数据覆盖 name/name_en，但 name_zh/aliases 不会被覆盖
    - DLC 歌曲不从 Data/mdata_pv_db.txt 导入；DLC/MOD 歌曲统一从 mods 目录扫描
    """

    def __init__(self, db_path: str | Path):
        self.db_path = Path(db_path)
        self.songs: dict[int, SongEntry] = {}  # pv_id -> SongEntry

    # ------------------------------------------------------------------ #
    #  加载 / 保存                                                          #
    # ------------------------------------------------------------------ #

    def load(self) -> bool:
        """加载数据库，返回是否成功"""
        if not self.db_path.exists():
            return False
        try:
            with open(self.db_path, "r", encoding="utf-8") as f:
                raw = json.load(f)
            if raw.get("version", 0) != DB_VERSION:
                safe_print(f"警告: song_db.json 版本不匹配（期望 {DB_VERSION}，实际 {raw.get('version')}）")
            songs = raw.get("songs", {}) if isinstance(raw, dict) else {}
            if isinstance(songs, dict):
                for song_dict in songs.values():
                    if isinstance(song_dict, dict):
                        entry = SongEntry.from_dict(song_dict)
                        self.songs[entry.pv_id] = entry
            return True
        except Exception as e:
            safe_print(f"加载 song_db.json 失败: {e}")
            return False

    def save(self):
        """保存数据库"""
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        raw = {
            "version": DB_VERSION,
            "songs": {
                str(pv_id): entry.to_dict()
                for pv_id, entry in sorted(self.songs.items())
            },
        }
        with open(self.db_path, "w", encoding="utf-8") as f:
            json.dump(raw, f, ensure_ascii=False, indent=2)

    # ------------------------------------------------------------------ #
    #  从 pv_db 原始格式导入                                                #
    # ------------------------------------------------------------------ #

    def import_from_pvdb(
        self,
        file_path: str | Path,
        source: str = "base",
        overwrite_names: bool = True,
    ):
        """
        从 pv_db.txt / mod_pv_db.txt 导入歌曲。

        overwrite_names=True 时，若 pv_id 已存在，更新 name/name_en/authors/difficulty。
        name_zh 和 aliases 永远不被覆盖（由用户/翻译工具管理）。
        """
        file_path = Path(file_path)
        lines = file_path.read_text(encoding="UTF-8").splitlines()
        # 按 pv_id 归组
        entries: dict[int, dict[str, str]] = {}
        for line in lines:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            # pv_NNN.key=value
            if "=" not in line or "." not in line:
                continue
            dot_idx = line.index(".")
            eq_idx = line.index("=")
            pv_part = line[:dot_idx]  # "pv_001"
            key_part = line[dot_idx + 1 : eq_idx]  # "song_name"
            val_part = line[eq_idx + 1 :]  # value

            if not pv_part.startswith("pv_"):
                continue
            id_str = pv_part[3:]
            if not id_str.isdigit():
                continue
            pv_id = int(id_str)
            entries.setdefault(pv_id, {})[key_part] = val_part

        for pv_id, fields in entries.items():
            name = fields.get("song_name", "")
            name_en = fields.get("song_name_en", "")

            # 解析难度
            difficulty = self._parse_difficulty(fields)

            # 解析作者（music 字段为主，回退到 songinfo_en.music）
            music = (
                fields.get("songinfo.music", "")
                or fields.get("songinfo_en.music", "")
            ).strip()
            authors = [music] if music else []

            if pv_id in self.songs:
                # 已存在：按策略更新
                existing = self.songs[pv_id]
                if overwrite_names:
                    if name:
                        existing.name = name
                    if name_en:
                        existing.name_en = name_en
                    if authors:
                        existing.authors = authors
                    if difficulty:
                        existing.difficulty = difficulty
                existing.source = source
            else:
                self.songs[pv_id] = SongEntry(
                    pv_id=pv_id,
                    name=name,
                    name_en=name_en,
                    authors=authors,
                    difficulty=difficulty,
                    source=source,
                )

    def _parse_difficulty(self, fields: dict[str, str]) -> dict[str, float]:
        """从 pv_db 字段字典中解析难度信息"""
        result: dict[str, float] = {}

        # 难度名映射（pv_db 中的 key → 我们的 key）
        diff_map = {
            "easy": "easy",
            "normal": "normal",
            "hard": "hard",
            "extreme": "extreme",
        }

        for db_key, our_key in diff_map.items():
            level = fields.get(f"difficulty.{db_key}.0.level", "")
            if level:
                result[our_key] = parse_level(level)

        # exextreme：extreme edition=1 即为 exex
        exex_level = fields.get("difficulty.extreme.1.level", "")
        if exex_level:
            result["exextreme"] = parse_level(exex_level)

        return result

    # ------------------------------------------------------------------ #
    #  别名管理                                                             #
    # ------------------------------------------------------------------ #

    def import_aliases(self, alias_file: str | Path):
        """
        从 AnotherSongName.json 导入别名。
        格式: { "别名": "原文歌曲名" }
        将别名挂载到匹配 name 或 name_en 的歌曲上。
        未能匹配的别名打印警告。
        """
        alias_file = Path(alias_file)
        if not alias_file.exists():
            safe_print(f"警告: 别名文件不存在: {alias_file}")
            return

        with open(alias_file, "r", encoding="UTF-8") as f:
            alias_dict: dict[str, str] = json.load(f)

        # 建立 name/name_en → pv_id 的快速索引
        name_index: dict[str, int] = {}
        for pv_id, entry in self.songs.items():
            if entry.name:
                name_index[entry.name.lower()] = pv_id
            if entry.name_en:
                name_index[entry.name_en.lower()] = pv_id

        matched = 0
        unmatched = []
        for alias, target_name in alias_dict.items():
            target_lower = target_name.lower()
            pv_id = name_index.get(target_lower)
            if pv_id is not None:
                entry = self.songs[pv_id]
                if alias not in entry.aliases:
                    entry.aliases.append(alias)
                matched += 1
            else:
                unmatched.append(f"{alias!r} → {target_name!r}")

        safe_print(f"别名导入: {matched} 个匹配，{len(unmatched)} 个未匹配")
        if unmatched:
            for item in unmatched:
                safe_print(f"  未匹配: {item}")

    # ------------------------------------------------------------------ #
    #  中文名更新                                                           #
    # ------------------------------------------------------------------ #

    def update_chinese_names(self, name_cache: dict[str, str]):
        """
        将 {日文名: 中文名} 缓存写入对应歌曲的 name_zh 字段。
        只更新 name_zh 为空的条目（不覆盖已有翻译）。
        返回更新数量。
        """
        updated = 0
        for entry in self.songs.values():
            if not entry.name_zh and entry.name in name_cache:
                entry.name_zh = name_cache[entry.name]
                updated += 1
            elif not entry.name_zh and entry.name_en in name_cache:
                entry.name_zh = name_cache[entry.name_en]
                updated += 1
        return updated

    def clear_chinese_names(self):
        """从 build 好的 song_db 中移除中文名字段，仅保留运行时内存状态清理。"""
        for entry in self.songs.values():
            entry.name_zh = ""

    # ------------------------------------------------------------------ #
    #  统计                                                                 #
    # ------------------------------------------------------------------ #

    def stats(self) -> dict[str, object]:
        total = len(self.songs)
        with_zh = sum(1 for e in self.songs.values() if e.name_zh)
        with_aliases = sum(1 for e in self.songs.values() if e.aliases)
        sources: dict[str, int] = {}
        for e in self.songs.values():
            src = e.source if not e.source.startswith("mod:") else "mod"
            sources[src] = sources.get(src, 0) + 1
        return {
            "total": total,
            "with_zh": with_zh,
            "with_aliases": with_aliases,
            "sources": sources,
        }


class ChineseNameDatabase:
    """
    独立中文名数据库（Data/song_name_zh.json）。

    格式：
    {
      "version": 2,
      "names": {
        "恋は戦争": "恋爱战争"
      }
    }

    key 为原曲名（日文/原文名，来自 song_db.json 的 name 字段），
    不使用 pvid，避免 pvid 在不同 mod 间重复导致对应关系错误。
    """

    VERSION = 2

    def __init__(self, db_path: str | Path):
        self.db_path = Path(db_path)
        self.names: dict[str, str] = {}  # 原曲名 -> 中文名

    def load(self) -> bool:
        if not self.db_path.exists():
            return False
        try:
            with open(self.db_path, "r", encoding="utf-8") as f:
                raw = json.load(f)

            raw_names: object
            if isinstance(raw, dict) and "names" in raw:
                raw_names = raw.get("names", {})
            else:
                raw_names = raw

            if not isinstance(raw_names, dict):
                return False

            self.names = {}
            for key, value in raw_names.items():
                if isinstance(key, str) and key and isinstance(value, str) and value:
                    self.names[key] = value
            return True
        except Exception as e:
            safe_print(f"加载中文名数据库失败: {e}")
            return False

    def save(self):
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        raw = {
            "version": self.VERSION,
            "names": {name: zh for name, zh in sorted(self.names.items())},
        }
        with open(self.db_path, "w", encoding="utf-8") as f:
            json.dump(raw, f, ensure_ascii=False, indent=2)

    def update_from_song_db(self, song_db: SongDatabase) -> int:
        """从 song_db.json 的 name_zh 字段合并到中文名库（按原曲名存储）。"""
        updated = 0
        for entry in song_db.songs.values():
            if entry.name_zh and entry.name and entry.name not in self.names:
                self.names[entry.name] = entry.name_zh
                updated += 1
        return updated

    def update_from_name_cache(self, song_db: SongDatabase, name_cache: dict[str, str]) -> int:
        """从旧版 {原文名/英文名: 中文名} 缓存合并到中文名库。"""
        updated = 0
        for entry in song_db.songs.values():
            if not entry.name or entry.name in self.names:
                continue
            chinese_name = name_cache.get(entry.name) or name_cache.get(entry.name_en)
            if chinese_name:
                self.names[entry.name] = chinese_name
                updated += 1
        return updated

    def apply_to_song_db(self, song_db: SongDatabase):
        """按原曲名将中文名叠加到 SongDatabase 的内存对象上。"""
        for entry in song_db.songs.values():
            if entry.name and entry.name in self.names:
                entry.name_zh = self.names[entry.name]
