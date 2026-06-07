"""
配置管理模块
"""

import json
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import cast


@dataclass
class Config:
    """应用配置"""
    
    # B站直播间ID
    room_id: int = 0
    
    # 切歌快捷键
    hotkey: str = "ctrl+shift+n"
    
    # 数据目录
    data_dir: str = "Data"
    
    # MOD文件夹路径（游戏mods目录）
    mods_dir: str = ""
    
    # 是否自动播放队列中的下一首
    auto_play_next: bool = False
    
    # 自动播放间隔（秒）
    auto_play_interval: int = 300
    
    # 最大队列长度
    max_queue_size: int = 50
    
    # 是否允许重复点歌
    allow_duplicates: bool = False

    # 是否启用 OBS 浏览器源点歌队列组件
    obs_overlay_enabled: bool = True

    # OBS 浏览器源监听地址
    obs_overlay_host: str = "127.0.0.1"

    # OBS 浏览器源监听端口
    obs_overlay_port: int = 8765

    # OBS 组件标题
    obs_overlay_title: str = "点歌队列"
    
    # 点歌命令前缀
    song_command_prefix: str = "点歌"
    
    # B站SESSDATA（可选，用于获取完整用户名）
    sessdata: str = ""
    
    # HTTP代理（用于访问VocaDB API）
    http_proxy: str = ""

    # 难度搜索默认使用的难度档（用于难度星级过滤）
    # 可选值: "easy" / "normal" / "hard" / "extreme" / "exextreme"
    default_search_difficulty: str = "extreme"

    # 难度过滤的容差范围（±星级），如 7.0 ±0.5 表示匹配 6.5~7.5
    difficulty_tolerance: float = 0.5
    
    # ---- LLM 意图识别配置 ----
    
    # 是否启用LLM意图识别（关闭时回退到前缀匹配）
    llm_enabled: bool = False
    
    # LLM API Key
    llm_api_key: str = ""
    
    # LLM API Base URL（支持OpenAI兼容接口）
    # DeepSeek:  https://api.deepseek.com
    # 通义千问:  https://dashscope.aliyuncs.com/compatible-mode/v1
    # 智谱GLM:   https://open.bigmodel.cn/api/paas/v4/
    # OpenAI:    https://api.openai.com/v1
    llm_base_url: str = "https://api.deepseek.com"
    
    # LLM 模型名称
    llm_model: str = "deepseek-chat"
    
    # 配置文件路径
    config_file: str = "config.json"
    
    def load(self, config_path: str | None = None):
        """加载配置文件"""
        if config_path:
            self.config_file = config_path

        path = Path(self.config_file)
        if path.exists():
            with open(path, "r", encoding="utf-8") as f:
                raw_data = cast(object, json.load(f))
                if not isinstance(raw_data, dict):
                    return
                data = cast(dict[object, object], raw_data)
                for key, value in data.items():
                    if not isinstance(key, str):
                        continue
                    if hasattr(self, key):
                        setattr(self, key, value)

    def save(self, config_path: str | None = None):
        """保存配置文件"""
        if config_path:
            self.config_file = config_path

        path = Path(self.config_file)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(asdict(self), f, indent=2, ensure_ascii=False)
    
    def validate(self) -> bool:
        """验证配置"""
        if not self.room_id:
            print("错误: 未设置直播间ID")
            return False
        
        if not self.hotkey:
            print("错误: 未设置快捷键")
            return False

        if self.obs_overlay_enabled and self.obs_overlay_host not in {"127.0.0.1", "localhost"}:
            print("错误: OBS点歌队列组件仅支持本机地址 127.0.0.1 或 localhost")
            return False
        
        return True
