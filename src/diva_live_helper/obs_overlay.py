"""
OBS 浏览器源点歌队列展示服务。
"""

from __future__ import annotations

import asyncio
import json
import time
from collections.abc import Awaitable, Callable
from html import escape

from diva_live_helper.queue import SongQueue


class OBSOverlayServer:
    """为 OBS 浏览器源提供本地网页和队列 JSON 接口。"""

    def __init__(
        self,
        song_queue: SongQueue,
        host: str = "127.0.0.1",
        port: int = 8765,
        title: str = "点歌队列",
    ):
        self.song_queue: SongQueue = song_queue
        self.host: str = host
        self.port: int = port
        self.title: str = title
        self._server: asyncio.Server | None = None

    @property
    def url(self) -> str:
        """OBS 浏览器源地址。"""
        return f"http://{self.host}:{self.port}/"

    async def start(self) -> None:
        """启动本地 HTTP 服务。"""
        if self._server is not None:
            return
        self._server = await asyncio.start_server(
            self._handle_client,
            host=self.host,
            port=self.port,
        )

    async def stop(self) -> None:
        """停止本地 HTTP 服务。"""
        if self._server is None:
            return
        self._server.close()
        await self._server.wait_closed()
        self._server = None

    async def _handle_client(
        self,
        reader: asyncio.StreamReader,
        writer: asyncio.StreamWriter,
    ) -> None:
        try:
            request_line = await reader.readline()
            if not request_line:
                writer.close()
                await writer.wait_closed()
                return

            method, raw_path, _ = request_line.decode("iso-8859-1").split(maxsplit=2)
            await self._discard_headers(reader)

            if method != "GET":
                await self._send_response(writer, 405, "text/plain; charset=utf-8", b"Method Not Allowed")
                return

            path = raw_path.split("?", 1)[0]
            routes: dict[str, Callable[[], Awaitable[tuple[str, bytes]]]] = {
                "/": self._render_overlay,
                "/index.html": self._render_overlay,
                "/api/queue": self._render_queue_json,
            }
            handler = routes.get(path)
            if handler is None:
                await self._send_response(writer, 404, "text/plain; charset=utf-8", b"Not Found")
                return

            content_type, body = await handler()
            await self._send_response(
                writer,
                200,
                content_type,
                body,
                allow_cors=path == "/api/queue",
            )
        except (ValueError, UnicodeDecodeError):
            await self._send_response(writer, 400, "text/plain; charset=utf-8", b"Bad Request")
        finally:
            writer.close()
            await writer.wait_closed()

    async def _discard_headers(self, reader: asyncio.StreamReader) -> None:
        while True:
            line = await reader.readline()
            if not line or line in {b"\r\n", b"\n"}:
                return

    async def _render_queue_json(self) -> tuple[str, bytes]:
        snapshot = self.song_queue.snapshot()
        snapshot["updated_at"] = time.time()
        body = json.dumps(snapshot, ensure_ascii=False).encode("utf-8")
        return "application/json; charset=utf-8", body

    async def _render_overlay(self) -> tuple[str, bytes]:
        body = _OVERLAY_HTML.format(title=escape(self.title)).encode("utf-8")
        return "text/html; charset=utf-8", body

    async def _send_response(
        self,
        writer: asyncio.StreamWriter,
        status: int,
        content_type: str,
        body: bytes,
        allow_cors: bool = False,
    ) -> None:
        reason = {
            200: "OK",
            400: "Bad Request",
            404: "Not Found",
            405: "Method Not Allowed",
        }.get(status, "OK")
        cors_header = "Access-Control-Allow-Origin: *\r\n" if allow_cors else ""
        header = (
            f"HTTP/1.1 {status} {reason}\r\n"
            f"Content-Type: {content_type}\r\n"
            "Cache-Control: no-store\r\n"
            f"{cors_header}"
            f"Content-Length: {len(body)}\r\n"
            "Connection: close\r\n"
            "\r\n"
        ).encode("ascii")
        writer.write(header + body)
        await writer.drain()


_OVERLAY_HTML = r"""<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>
    :root {{
      --bg: rgba(5, 10, 18, 0.72);
      --panel: rgba(10, 20, 34, 0.78);
      --cyan: #32f6ff;
      --pink: #ff4fd8;
      --gold: #ffe27a;
      --text: #f7fbff;
      --muted: rgba(247, 251, 255, 0.68);
      --line: rgba(50, 246, 255, 0.22);
      font-family: "Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI", sans-serif;
    }}

    * {{ box-sizing: border-box; }}

    body {{
      margin: 0;
      min-height: 100vh;
      color: var(--text);
      background: transparent;
      overflow: hidden;
    }}

    .overlay {{
      width: min(680px, calc(100vw - 32px));
      margin: 16px;
      padding: 18px;
      border: 1px solid var(--line);
      border-radius: 24px;
      background:
        radial-gradient(circle at 12% 0%, rgba(255, 79, 216, 0.24), transparent 32%),
        radial-gradient(circle at 88% 12%, rgba(50, 246, 255, 0.26), transparent 34%),
        linear-gradient(135deg, var(--bg), var(--panel));
      box-shadow: 0 24px 80px rgba(0, 0, 0, 0.42), inset 0 0 34px rgba(50, 246, 255, 0.08);
      backdrop-filter: blur(14px);
      position: relative;
    }}

    .overlay::before {{
      content: "";
      position: absolute;
      inset: 10px;
      pointer-events: none;
      border-radius: 18px;
      border: 1px solid rgba(255, 255, 255, 0.06);
    }}

    header {{
      display: grid;
      grid-template-columns: 1fr auto;
      gap: 12px;
      align-items: end;
      margin-bottom: 14px;
    }}

    .eyebrow {{
      color: var(--cyan);
      font-size: 12px;
      letter-spacing: 0.28em;
      text-transform: uppercase;
      text-shadow: 0 0 14px rgba(50, 246, 255, 0.8);
    }}

    h1 {{
      margin: 2px 0 0;
      font-size: 34px;
      line-height: 1;
      letter-spacing: 0.04em;
      text-shadow: 3px 3px 0 rgba(255, 79, 216, 0.72), 0 0 24px rgba(50, 246, 255, 0.42);
    }}

    .counter {{
      min-width: 92px;
      padding: 10px 12px;
      border-radius: 16px;
      background: rgba(0, 0, 0, 0.24);
      border: 1px solid rgba(255, 226, 122, 0.28);
      text-align: center;
      color: var(--gold);
      box-shadow: inset 0 0 18px rgba(255, 226, 122, 0.08);
    }}

    .counter strong {{ display: block; font-size: 26px; line-height: 1; }}
    .counter span {{ font-size: 11px; color: var(--muted); }}

    .list {{
      display: grid;
      gap: 10px;
      max-height: calc(100vh - 150px);
      overflow: hidden;
    }}

    .song {{
      display: grid;
      grid-template-columns: 44px 1fr auto;
      gap: 12px;
      align-items: center;
      padding: 12px 14px;
      border-radius: 18px;
      background: linear-gradient(90deg, rgba(255, 255, 255, 0.11), rgba(255, 255, 255, 0.045));
      border: 1px solid rgba(255, 255, 255, 0.08);
      animation: slide-in 360ms ease both;
    }}

    .song:first-child {{
      background: linear-gradient(90deg, rgba(50, 246, 255, 0.24), rgba(255, 79, 216, 0.12));
      border-color: rgba(50, 246, 255, 0.34);
    }}

    .pos {{
      width: 38px;
      height: 38px;
      display: grid;
      place-items: center;
      border-radius: 14px;
      color: #061019;
      background: linear-gradient(135deg, var(--cyan), var(--gold));
      font-weight: 900;
      box-shadow: 0 0 18px rgba(50, 246, 255, 0.35);
    }}

    .name {{
      font-size: 20px;
      font-weight: 800;
      letter-spacing: 0.02em;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}

    .meta {{
      margin-top: 3px;
      color: var(--muted);
      font-size: 13px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}

    .difficulty {{
      padding: 7px 10px;
      border-radius: 999px;
      color: var(--pink);
      background: rgba(255, 79, 216, 0.12);
      border: 1px solid rgba(255, 79, 216, 0.22);
      font-weight: 800;
      white-space: nowrap;
    }}

    .empty {{
      padding: 34px 18px;
      border-radius: 18px;
      border: 1px dashed rgba(50, 246, 255, 0.28);
      color: var(--muted);
      text-align: center;
      background: rgba(0, 0, 0, 0.18);
    }}

    .empty strong {{ color: var(--cyan); display: block; font-size: 22px; margin-bottom: 6px; }}

    @keyframes slide-in {{
      from {{ opacity: 0; transform: translateX(-16px) scale(0.98); }}
      to {{ opacity: 1; transform: translateX(0) scale(1); }}
    }}
  </style>
</head>
<body>
  <main class="overlay">
    <header>
      <div>
        <div class="eyebrow">Project DIVA Live Helper</div>
        <h1>{title}</h1>
      </div>
      <div class="counter"><strong id="queue-size">0</strong><span>WAITING</span></div>
    </header>
    <section id="queue-list" class="list" aria-live="polite"></section>
  </main>

  <script>
    const list = document.getElementById('queue-list');
    const counter = document.getElementById('queue-size');

    function escapeHtml(value) {{
      return String(value ?? '').replace(/[&<>"']/g, (char) => ({{
        '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
      }}[char]));
    }}

    function render(data) {{
      counter.textContent = data.size ?? 0;
      const songs = data.songs ?? [];
      if (songs.length === 0) {{
        list.innerHTML = '<div class="empty"><strong>队列待机中</strong>发送弹幕点歌后会实时显示在这里</div>';
        return;
      }}

      list.innerHTML = songs.map((song) => {{
        const difficulty = song.difficulty ? `<div class="difficulty">★ ${{escapeHtml(song.difficulty)}}</div>` : '';
        return `<article class="song">
          <div class="pos">${{escapeHtml(song.position)}}</div>
          <div>
            <div class="name">${{escapeHtml(song.song_name)}}</div>
            <div class="meta">点歌人：${{escapeHtml(song.requester)}} · PV ${{escapeHtml(song.song_id)}}</div>
          </div>
          ${{difficulty}}
        </article>`;
      }}).join('');
    }}

    async function refresh() {{
      try {{
        const response = await fetch('/api/queue', {{ cache: 'no-store' }});
        if (!response.ok) throw new Error(`HTTP ${{response.status}}`);
        render(await response.json());
      }} catch (error) {{
        list.innerHTML = '<div class="empty"><strong>连接中断</strong>等待 diva-live-helper 队列服务恢复</div>';
      }}
    }}

    refresh();
    setInterval(refresh, 1000);
  </script>
</body>
</html>"""
