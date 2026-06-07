"""
B站弹幕处理模块
使用blivedm库连接B站直播间
"""

import asyncio
import http.cookies
from collections.abc import Callable

import aiohttp
import blivedm


class DanmakuHandler(blivedm.BaseHandler):
    """弹幕消息处理器"""
    
    def __init__(self, callback: Callable[[str, str], None]):
        """
        初始化处理器
        
        Args:
            callback: 弹幕回调函数，参数为 (用户名, 弹幕内容)
        """
        self.callback = callback
    
    async def _on_danmaku(self, client: blivedm.BLiveClient, message: blivedm.DanmakuMessage):
        """处理弹幕消息"""
        try:
            self.callback(message.uname, message.msg)
        except Exception as e:
            print(f"处理弹幕消息出错: {e}")


class DanmakuClient:
    """B站弹幕客户端"""
    
    def __init__(self, room_id: int, callback: Callable[[str, str], None], sessdata: str = ""):
        """
        初始化弹幕客户端
        
        Args:
            room_id: 直播间ID
            callback: 弹幕回调函数，参数为 (用户名, 弹幕内容)
            sessdata: B站SESSDATA（可选，用于获取完整用户名）
        """
        self.room_id = room_id
        self.callback = callback
        self.sessdata = sessdata
        self.client = None
        self.session = None
        self.handler = None
        self.running = False
    
    async def start(self):
        """启动弹幕客户端"""
        print(f"正在连接直播间 {self.room_id}...")
        
        # 创建HTTP会话（禁用brotli压缩以避免兼容性问题）
        self.session = aiohttp.ClientSession(
            headers={"Accept-Encoding": "gzip, deflate"}
        )
        
        # 设置cookies（如果提供了SESSDATA）
        if self.sessdata:
            cookies = http.cookies.SimpleCookie()
            cookies['SESSDATA'] = self.sessdata
            cookies['SESSDATA']['domain'] = 'bilibili.com'
            self.session.cookie_jar.update_cookies(cookies)
        
        # 创建弹幕客户端
        self.client = blivedm.BLiveClient(self.room_id, session=self.session)
        
        # 创建消息处理器
        self.handler = DanmakuHandler(self.callback)
        self.client.add_handler(self.handler)
        
        # 启动客户端
        self.client.start()
        self.running = True
        
        print(f"已连接到直播间 {self.room_id}")
        
        # 启动心跳检测
        asyncio.create_task(self._heartbeat_loop())
    
    async def stop(self):
        """停止弹幕客户端"""
        self.running = False
        
        if self.client:
            self.client.stop()
            await self.client.join()
            await self.client.stop_and_close()
        
        if self.session:
            await self.session.close()
        
        print("弹幕客户端已停止")
    
    async def _heartbeat_loop(self):
        """心跳检测循环"""
        while self.running:
            try:
                await asyncio.sleep(30)
                # blivedm内部会处理心跳，这里只是保持连接活跃
            except Exception as e:
                print(f"心跳检测出错: {e}")
                await asyncio.sleep(1)
    
    def is_connected(self) -> bool:
        """检查是否已连接"""
        return self.running and self.client is not None


class MockDanmakuClient(DanmakuClient):
    """模拟弹幕客户端（用于测试）"""
    
    def __init__(self, room_id: int, callback: Callable[[str, str], None]):
        super().__init__(room_id, callback)
        self.test_messages = [
            ("用户1", "点歌 Love is War"),
            ("用户2", "点歌 初音未来的消失"),
            ("用户3", "大家好"),
            ("用户4", "点歌 みくみくにしてあげる♪"),
            ("用户5", "点歌 39"),
        ]
    
    async def start(self):
        """启动模拟客户端"""
        print(f"模拟连接到直播间 {self.room_id}")
        self.running = True
        
        # 启动模拟消息发送
        asyncio.create_task(self._send_test_messages())
    
    async def _send_test_messages(self):
        """发送测试消息"""
        import random
        
        while self.running:
            # 随机选择一条测试消息
            username, message = random.choice(self.test_messages)
            
            # 调用回调
            try:
                self.callback(username, message)
            except Exception as e:
                print(f"处理模拟消息出错: {e}")
            
            # 等待随机时间
            await asyncio.sleep(random.uniform(2, 5))
