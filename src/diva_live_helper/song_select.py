"""
歌曲选择模块
从fast_jump_en.py中提取并重构
"""

import time

import pymem
from pymem.exception import ProcessNotFound


class SongSelector:
    """歌曲选择器 - 通过修改游戏内存切换歌曲"""
    
    def __init__(self, song_searcher=None):
        self.pm = None
        self.base_address = 0
        self.song_searcher = song_searcher
        
        # 内存地址偏移
        self.LastSelectPVIDMem = 0
        self.LastSelectSortMem = 0
        self.LastSelectDiffMem = 0
        self.EdenOffsetMem = 0x105F460
        self.ChangeSongSelect = 0
        self.StartChange = 0
        
        # 初始化
        self._initialize()
    
    def _initialize(self):
        """初始化游戏内存连接"""
        try:
            self.pm = pymem.Pymem('DivaMegaMix.exe')
            self.base_address = self.pm.base_address
            
            # 设置内存地址
            self.LastSelectPVIDMem = self.base_address + 0x12B6350
            self.LastSelectSortMem = self.base_address + 0x12B6354
            self.LastSelectDiffMem = self.base_address + 0x12B635C
            self.ChangeSongSelect = self.base_address + 0xCC61098
            self.StartChange = self.base_address + 0xCC610A0
            
            # 检查Eden偏移
            self._check_eden()
            
            print("已连接到游戏进程")
        except ProcessNotFound:
            print("警告: 游戏进程未启动，请先启动游戏")
            self.pm = None
    
    def _check_eden(self):
        """检查是否需要Eden偏移"""
        pm = self.pm
        if pm is None:
            return
        if pm.read_int(self.LastSelectPVIDMem) == 0:
            self.LastSelectPVIDMem += self.EdenOffsetMem
            self.LastSelectSortMem += self.EdenOffsetMem
            self.LastSelectDiffMem += self.EdenOffsetMem
    
    def is_connected(self) -> bool:
        """检查是否已连接到游戏"""
        return self.pm is not None
    
    def reconnect(self) -> bool:
        """重新连接到游戏"""
        try:
            self._initialize()
            return self.is_connected()
        except Exception as e:
            print(f"重新连接失败: {e}")
            return False
    
    def change_song(self, song_id: int) -> str:
        """
        切换歌曲
        
        Args:
            song_id: 歌曲ID
        
        Returns:
            操作结果: "success!" 或 错误信息
        """
        if not self.is_connected():
            return "游戏进程未连接"

        pm = self.pm
        if pm is None:
            return "游戏进程未连接"
        
        try:
            song_id = int(song_id)
        except ValueError:
            return "无效的歌曲ID"
        
        # 检查歌曲ID是否存在（仅当注入了已加载的 searcher 才校验）
        if self.song_searcher and not self.song_searcher.check_id(song_id):
            return "歌曲ID不存在"
        
        try:
            # 等待游戏状态稳定
            time.sleep(0.1)
            
            # 检查游戏状态
            if pm.read_int(self.ChangeSongSelect) == 6:
                # 游戏在选歌界面
                pm.write_int(self.ChangeSongSelect, 6)
                pm.write_int(self.StartChange, 2)
                time.sleep(0.1)

                # 设置歌曲ID
                pm.write_int(self.LastSelectPVIDMem, song_id)

                # 设置排序方式为难度分类
                pm.write_int(self.LastSelectSortMem, 1)

                # 设置难度为ALL
                pm.write_int(self.LastSelectDiffMem, 19)

                # 触发歌曲切换
                pm.write_int(self.ChangeSongSelect, 5)
                pm.write_int(self.StartChange, 2)
            else:
                # 游戏不在选歌界面，直接设置
                pm.write_int(self.LastSelectPVIDMem, song_id)
                pm.write_int(self.LastSelectSortMem, 1)
                pm.write_int(self.LastSelectDiffMem, 19)
            
            return "success!"
        except Exception as e:
            return f"切换歌曲时出错: {e}"
    
    def get_current_song_id(self) -> int | None:
        """获取当前选中的歌曲ID"""
        if not self.is_connected():
            return None

        pm = self.pm
        if pm is None:
            return None

        try:
            return int(pm.read_int(self.LastSelectPVIDMem))
        except Exception:
            return None
