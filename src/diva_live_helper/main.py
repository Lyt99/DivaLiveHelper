#!/usr/bin/env python3
"""
diva-live-helper - B站直播点歌助手
主入口文件
"""

import asyncio
import re
import sys
import signal
from pathlib import Path
from types import FrameType

# 添加项目根目录到Python路径
project_root = Path(__file__).parent.parent.parent
sys.path.insert(0, str(project_root))

from diva_live_helper.config import Config
from diva_live_helper.danmaku import DanmakuClient
from diva_live_helper.song_search import SongSearcher, DIFFICULTY_KEY_ALIASES
from diva_live_helper.song_select import SongSelector
from diva_live_helper.queue import SongQueue
from diva_live_helper.hotkey import HotkeyListener
from diva_live_helper.llm_intent import LLMIntentAnalyzer
from diva_live_helper.obs_overlay import OBSOverlayServer


# 前缀点歌时从弹幕末尾解析难度后缀
# 示例: "点歌 恋は戦争 ex" → song="恋は戦争", diff_key="extreme"
#       "点歌 恋は戦争 7星"  → song="恋は戦争", diff_level=7.0
_DIFFICULTY_SUFFIX_RE = re.compile(
    r"""
    \s+                             # 与歌名之间有空格
    (?:
        (\d+(?:\.\d+)?)\s*星        # 数字星级，如 "7星" "8.5星"
      | (exex|exextreme|ex2|encore  # 难度档别名
         |extreme|extr|ex
         |hard|normal|easy
         |[enhH])                   # 单字母缩写
    )
    $
    """,
    re.VERBOSE | re.IGNORECASE,
)


def _parse_prefix_difficulty(text: str) -> tuple[str, float | None]:
    """
    从前缀点歌的歌名部分尝试分离末尾难度关键词。

    返回 (cleaned_song_name, difficulty_level_or_None)
    difficulty 已转换为 extreme 档对应的目标星级浮点；
    若是数字星级则直接用该数字；若未指定则返回 None。
    """
    m = _DIFFICULTY_SUFFIX_RE.search(text)
    if not m:
        return text.strip(), None

    song_name = text[: m.start()].strip()

    if m.group(1):
        # 数字星级
        return song_name, float(m.group(1))

    # 难度档 → 代表性星级中值（仅用于过滤）
    diff_word = m.group(2).lower()
    diff_key = DIFFICULTY_KEY_ALIASES.get(diff_word)
    if diff_key is None:
        return text.strip(), None

    # 各档典型中值（用于过滤时的 target）
    _DIFF_MIDPOINTS = {
        "easy": 3.0,
        "normal": 5.0,
        "hard": 7.0,
        "extreme": 9.0,
        "exextreme": 9.5,
    }
    return song_name, _DIFF_MIDPOINTS.get(diff_key)


class DivaLiveHelper:
    """主应用类"""

    def __init__(self):
        self.config: Config = Config()
        self.config.load()
        self.song_searcher: SongSearcher = SongSearcher(
            data_dir=self.config.data_dir,
            mods_dir=self.config.mods_dir,
            proxy=self.config.http_proxy,
        )
        self.song_selector: SongSelector = SongSelector(song_searcher=self.song_searcher)
        self.song_queue: SongQueue = SongQueue(
            max_size=self.config.max_queue_size,
            allow_duplicates=self.config.allow_duplicates,
        )
        self.danmaku_client: DanmakuClient | None = None
        self.hotkey_listener: HotkeyListener | None = None
        self.llm_analyzer: LLMIntentAnalyzer | None = None
        self.obs_overlay_server: OBSOverlayServer | None = None
        self.running: bool = False

    async def start(self):
        """启动应用"""
        print("正在启动 diva-live-helper...")

        if not self.config.validate():
            print("配置验证失败，请检查config.json文件")
            return

        if self.config.llm_enabled:
            self.llm_analyzer = LLMIntentAnalyzer(
                api_key=self.config.llm_api_key,
                base_url=self.config.llm_base_url,
                model=self.config.llm_model,
                proxy=self.config.http_proxy,
            )
            key_note = "未配置 API Key，本地模型模式" if not self.config.llm_api_key else "已配置 API Key"
            print(f"LLM意图分析已启用 (model: {self.config.llm_model}, {key_note})")

        print("加载歌曲数据库...")
        self.song_searcher.load_database()

        print("连接B站直播间...")
        self.danmaku_client = DanmakuClient(
            room_id=self.config.room_id,
            callback=self._on_danmaku,
            sessdata=self.config.sessdata,
        )

        print("注册快捷键...")
        self.hotkey_listener = HotkeyListener(
            hotkey=self.config.hotkey,
            callback=self._on_hotkey_pressed,
        )

        self.running = True
        try:
            if self.config.obs_overlay_enabled:
                self.obs_overlay_server = OBSOverlayServer(
                    song_queue=self.song_queue,
                    host=self.config.obs_overlay_host,
                    port=self.config.obs_overlay_port,
                    title=self.config.obs_overlay_title,
                )
                await self.obs_overlay_server.start()
                print(f"OBS点歌队列组件已启动: {self.obs_overlay_server.url}")

            await self.danmaku_client.start()
            self.hotkey_listener.start()

            print(f"已连接直播间 {self.config.room_id}")
            if self.llm_analyzer:
                print(f"点歌方式: LLM语义识别（也支持 {self.config.song_command_prefix} <歌名> 前缀）")
            else:
                print(f"点歌命令格式: {self.config.song_command_prefix} <歌名>")
            print(f"切歌快捷键: {self.config.hotkey}")
            print("按 Ctrl+C 退出...")

            while self.running:
                await asyncio.sleep(1)
        except KeyboardInterrupt:
            print("\n正在退出...")
        finally:
            await self.stop()

    async def stop(self):
        """停止应用"""
        self.running = False
        if self.danmaku_client:
            await self.danmaku_client.stop()
        if self.hotkey_listener:
            self.hotkey_listener.stop()
        if self.obs_overlay_server:
            await self.obs_overlay_server.stop()
        if self.llm_analyzer:
            await self.llm_analyzer.close()
        print("已退出")

    # ------------------------------------------------------------------ #
    #  弹幕处理                                                             #
    # ------------------------------------------------------------------ #

    def _on_danmaku(self, username: str, message: str):
        """处理弹幕消息（同步回调）"""
        print(f"[弹幕] {username}: {message}")

        if self.llm_analyzer:
            _ = asyncio.create_task(self._handle_danmaku_with_llm(username, message))
        else:
            self._handle_danmaku_with_prefix(username, message)

    def _handle_danmaku_with_prefix(self, username: str, message: str):
        """前缀匹配点歌逻辑"""
        prefix = self.config.song_command_prefix
        if not message.startswith(prefix):
            return
        raw = message[len(prefix):].strip()
        if not raw:
            return

        song_name, difficulty = _parse_prefix_difficulty(raw)
        if not song_name:
            return

        print(f"[前缀匹配] 收到点歌请求: {song_name}"
              + (f" (难度≈{difficulty})" if difficulty else ""))
        self._process_song_request(song_name, username, difficulty=difficulty)

    async def _handle_danmaku_with_llm(self, username: str, message: str):
        """LLM语义分析点歌逻辑（带前缀快速匹配优化）"""
        prefix = self.config.song_command_prefix

        # 前缀命中：直接解析，跳过 LLM
        if message.startswith(prefix):
            raw = message[len(prefix):].strip()
            if raw:
                song_name, difficulty = _parse_prefix_difficulty(raw)
                if song_name:
                    print(f"[前缀匹配] 收到点歌请求: {song_name}"
                          + (f" (难度≈{difficulty})" if difficulty else ""))
                    self._process_song_request(song_name, username, difficulty=difficulty)
            return

        # 非前缀：走 LLM
        analyzer = self.llm_analyzer
        if analyzer is None:
            return
        intent = await analyzer.analyze(message)
        if not intent.is_song_request:
            return

        difficulty = intent.difficulty

        if intent.author and not intent.song_name:
            # 纯作者点歌
            print(f"[LLM识别] 按作者点歌: {intent.author}"
                  + (f" (难度≈{difficulty})" if difficulty else ""))
            self._process_author_request(intent.author, username, difficulty=difficulty)
        elif intent.song_name:
            print(f"[LLM识别] 收到点歌请求: {intent.song_name}"
                  + (f" (难度≈{difficulty})" if difficulty else ""))
            self._process_song_request(
                intent.song_name, username,
                difficulty=difficulty,
            )

    # ------------------------------------------------------------------ #
    #  点歌处理                                                             #
    # ------------------------------------------------------------------ #

    def _process_song_request(
        self,
        song_name: str,
        requester: str,
        difficulty: float | None = None,
    ):
        """处理按歌名点歌"""
        results = self.song_searcher.search(
            song_name,
            difficulty=difficulty,
            difficulty_key=self.config.default_search_difficulty,
            difficulty_tolerance=self.config.difficulty_tolerance,
        )

        if not results:
            print(f"未找到匹配的歌曲: {song_name}")
            return

        pv_id, song_title = results[0]
        self._enqueue(pv_id, song_title, requester, difficulty)

    def _process_author_request(
        self,
        author: str,
        requester: str,
        difficulty: float | None = None,
    ):
        """处理按作者点歌（取匹配列表第一首）"""
        results = self.song_searcher.search_by_author(
            author,
            difficulty=difficulty,
            difficulty_key=self.config.default_search_difficulty,
            difficulty_tolerance=self.config.difficulty_tolerance,
        )

        if not results:
            print(f"未找到作者 [{author}] 的歌曲")
            return

        pv_id, song_title = results[0]
        print(f"[按作者] 找到 {len(results)} 首，取第一首: {song_title}")
        self._enqueue(pv_id, song_title, requester, difficulty)

    def _enqueue(
        self,
        pv_id: int,
        song_title: str,
        requester: str,
        difficulty: float | None,
    ):
        """将歌曲加入队列，打印结果"""
        added = self.song_queue.add(pv_id, song_title, requester, difficulty=difficulty)
        if added:
            diff_str = f", 难度≈{difficulty}" if difficulty else ""
            print(f"已添加到队列: {song_title} (ID: {pv_id}, 点歌人: {requester}{diff_str})")
            print(f"当前队列长度: {self.song_queue.size()}")
        else:
            print(f"添加失败（队列已满或歌曲重复）: {song_title}")

    # ------------------------------------------------------------------ #
    #  快捷键                                                               #
    # ------------------------------------------------------------------ #

    def _on_hotkey_pressed(self):
        """快捷键按下回调"""
        if self.song_queue.is_empty():
            print("队列为空，没有歌曲可播放")
            return

        next_request = self.song_queue.next()
        if next_request is None:
            print("队列为空，没有歌曲可播放")
            return

        pv_id, song_title, requester, difficulty = next_request
        diff_str = f" (难度≈{difficulty})" if difficulty else ""
        print(f"正在播放: {song_title}{diff_str} (点歌人: {requester})")

        result = self.song_selector.change_song(pv_id)
        if result == "success!":
            print(f"已切换到歌曲: {song_title}")
        else:
            print(f"切换失败: {result}")


def main():
    """主函数"""
    app = DivaLiveHelper()

    def signal_handler(sig: int, frame: FrameType | None):
        print(f"\n收到退出信号 ({sig})...")
        if frame is not None:
            pass
        app.running = False

    _ = signal.signal(signal.SIGINT, signal_handler)
    _ = signal.signal(signal.SIGTERM, signal_handler)

    asyncio.run(app.start())


if __name__ == "__main__":
    main()
