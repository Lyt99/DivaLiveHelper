"""
update_db — 更新歌曲数据库工具

独立运行，不依赖直播间连接。
用于扫描/重建结构化歌曲数据库 song_db.json。

用法：
    uv run update_db              # 仅扫描数据库，不查翻译
    uv run update_db --config path/to/config.json
"""

import argparse
import json
import sys
from pathlib import Path


def _load_name_cache(data_dir: str) -> dict[str, str]:
    """读取旧版中文名缓存 {原文名: 中文名}"""
    cache_file = Path(data_dir) / "song_name_cache.json"
    if not cache_file.exists():
        return {}
    try:
        with open(cache_file, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception as e:
        print(f"加载中文名缓存失败: {e}")
        return {}


def main():
    parser = argparse.ArgumentParser(
        prog="update_db",
        description="扫描并更新 DIVA 歌曲数据库（主库 / DLC / MOD）",
    )
    parser.add_argument(
        "--config",
        metavar="PATH",
        default="config.json",
        help="配置文件路径（默认 config.json）",
    )
    parser.add_argument(
        "--data-dir",
        metavar="DIR",
        default=None,
        help="数据文件目录（覆盖 config.json 中的 data_dir）",
    )
    parser.add_argument(
        "--mods-dir",
        metavar="DIR",
        default=None,
        help="游戏 mods 目录（覆盖 config.json 中的 mods_dir）",
    )

    args = parser.parse_args()

    # 加载配置
    from diva_live_helper.config import Config

    config = Config()
    config.load(args.config)

    # 命令行参数优先覆盖配置文件
    data_dir = args.data_dir or config.data_dir
    mods_dir = args.mods_dir or config.mods_dir

    print("=== update_db ===")
    print(f"数据目录  : {data_dir}")
    print(f"MOD 目录  : {mods_dir or '（未配置）'}")
    print()

    # 验证数据目录
    if not Path(data_dir).exists():
        print(f"错误: 数据目录不存在: {data_dir}", file=sys.stderr)
        sys.exit(1)

    from diva_live_helper.song_db import ChineseNameDatabase, SongDatabase

    data_path = Path(data_dir)
    db_path = data_path / "song_db.json"
    zh_db_path = data_path / "song_name_zh.json"
    db = SongDatabase(db_path)
    zh_db = ChineseNameDatabase(zh_db_path)
    zh_db.load()

    if db.load():
        print(f"已加载现有数据库: {db_path}")
        migrated = zh_db.update_from_song_db(db)
        if migrated:
            print(f"已从旧版 song_db.json 迁移中文名: {migrated} 条")
    else:
        print(f"将创建新数据库: {db_path}")

    # 导入主库。DLC 歌曲不再从 Data/mdata_pv_db.txt 读取，统一从 mods 目录扫描。
    pv_db_file = data_path / "pv_db.txt"
    if pv_db_file.exists():
        db.import_from_pvdb(pv_db_file, source="base")
        print(f"已导入主数据库: {pv_db_file}")
    else:
        print(f"警告: 主数据库不存在: {pv_db_file}")

    # 导入 MOD
    mods_path = Path(mods_dir) if mods_dir else None
    if mods_path and mods_path.exists():
        mod_count = 0
        for mod_folder in mods_path.iterdir():
            if not mod_folder.is_dir():
                continue
            mod_pv_db = mod_folder / "rom" / "mod_pv_db.txt"
            if not mod_pv_db.exists():
                continue
            db.import_from_pvdb(mod_pv_db, source=f"mod:{mod_folder.name}")
            mod_count += 1
            print(f"已导入MOD: {mod_folder.name}")
        print(f"MOD扫描完成: {mod_count} 个")

    # 兼容旧版 song_db.json：移除之前从 Data/mdata_pv_db.txt 导入且未被 base/MOD 覆盖的条目。
    removed_mdata = [pv_id for pv_id, entry in db.songs.items() if entry.source == "mdata"]
    for pv_id in removed_mdata:
        del db.songs[pv_id]
    if removed_mdata:
        print(f"已移除旧版 mdata DLC 条目: {len(removed_mdata)} 首")

    # 清理没有任何可搜索名称的残缺条目，避免 song_db 统计与运行时加载数量不一致。
    removed_unnamed = [
        pv_id
        for pv_id, entry in db.songs.items()
        if not entry.name and not entry.name_en and not entry.name_zh
    ]
    for pv_id in removed_unnamed:
        del db.songs[pv_id]
    if removed_unnamed:
        print(f"已移除无名称条目: {len(removed_unnamed)} 首")

    # 导入旧版别名文件到每首歌的 aliases 字段
    db.import_aliases(data_path / "AnotherSongName.json")

    # 合并旧版中文名缓存到独立中文名数据库（按 pvid 存储）
    name_cache = _load_name_cache(data_dir)
    if name_cache:
        updated = zh_db.update_from_name_cache(db, name_cache)
        print(f"已从 song_name_cache.json 合并中文名: {updated} 条")

    # build 好的 song_db 不保存中文名；中文名独立保存到 song_name_zh.json。
    db.clear_chinese_names()
    db.save()
    zh_db.save()
    stats = db.stats()

    print()
    print("数据库更新完成。")
    print(f"  保存路径  : {db_path}")
    print(f"  中文名库  : {zh_db_path}")
    print(f"  歌曲总数  : {stats['total']}")
    print(f"  中文名数  : {len(zh_db.names)}")
    print(f"  含别名    : {stats['with_aliases']}")
    print(f"  来源统计  : {stats['sources']}")


if __name__ == "__main__":
    main()
