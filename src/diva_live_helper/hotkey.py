"""
快捷键监听模块
"""

import threading
from collections.abc import Callable

try:
    import keyboard
except ImportError:
    keyboard = None
    print("警告: keyboard库未安装，快捷键功能将不可用")


class HotkeyListener:
    """快捷键监听器"""
    
    def __init__(self, hotkey: str, callback: Callable[[], None]):
        """
        初始化快捷键监听器
        
        Args:
            hotkey: 快捷键组合，如 "ctrl+shift+n"
            callback: 快捷键按下时的回调函数
        """
        self.hotkey = hotkey
        self.callback = callback
        self.listener_thread = None
        self.running = False
        
        # 验证keyboard库是否可用
        if keyboard is None:
            raise ImportError("keyboard库未安装，请运行: pip install keyboard")
    
    def start(self):
        """启动快捷键监听"""
        if self.running:
            print("快捷键监听已在运行")
            return
        
        self.running = True
        
        # 在后台线程中启动监听
        self.listener_thread = threading.Thread(
            target=self._listen_thread,
            daemon=True,
            name="HotkeyListener"
        )
        self.listener_thread.start()
        
        print(f"快捷键监听已启动: {self.hotkey}")
    
    def stop(self):
        """停止快捷键监听"""
        self.running = False

        # 移除热键
        try:
            keyboard_module = keyboard
            if keyboard_module is not None:
                keyboard_module.remove_hotkey(self.hotkey)
        except Exception:
            pass
        
        print("快捷键监听已停止")
    
    def _listen_thread(self):
        """监听线程"""
        try:
            keyboard_module = keyboard
            if keyboard_module is None:
                raise RuntimeError("keyboard库未安装")

            # 注册热键
            keyboard_module.add_hotkey(self.hotkey, self._on_hotkey_pressed)

            # 保持线程运行
            while self.running:
                keyboard_module.wait()
        except Exception as e:
            print(f"快捷键监听出错: {e}")
            self.running = False
    
    def _on_hotkey_pressed(self):
        """热键按下处理"""
        if self.running and self.callback:
            try:
                self.callback()
            except Exception as e:
                print(f"快捷键回调出错: {e}")
    
    def update_hotkey(self, new_hotkey: str):
        """
        更新快捷键
        
        Args:
            new_hotkey: 新的快捷键组合
        """
        # 停止当前监听
        self.stop()
        
        # 更新快捷键
        self.hotkey = new_hotkey
        
        # 重新启动监听
        self.start()
    
    def is_running(self) -> bool:
        """检查监听是否正在运行"""
        return self.running
