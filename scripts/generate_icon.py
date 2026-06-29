from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src-tauri" / "icons"
SIZE = 1024


SVG = """<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024" role="img" aria-label="DIVA直播助手图标">
  <defs>
    <linearGradient id="base" x1="128" y1="96" x2="896" y2="928" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#101824"/><stop offset="0.52" stop-color="#0d1117"/><stop offset="1" stop-color="#181126"/>
    </linearGradient>
    <radialGradient id="cyanGlow" cx="330" cy="270" r="430" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#39C5BB" stop-opacity="0.95"/><stop offset="1" stop-color="#39C5BB" stop-opacity="0"/></radialGradient>
    <radialGradient id="pinkGlow" cx="750" cy="770" r="430" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#E12885" stop-opacity="0.9"/><stop offset="1" stop-color="#E12885" stop-opacity="0"/></radialGradient>
    <linearGradient id="ribbon" x1="228" y1="722" x2="814" y2="302" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#39C5BB"/><stop offset="1" stop-color="#E12885"/></linearGradient>
    <filter id="softShadow" x="-30%" y="-30%" width="160%" height="160%"><feDropShadow dx="0" dy="34" stdDeviation="36" flood-color="#000000" flood-opacity="0.38"/></filter>
  </defs>
  <rect x="64" y="64" width="896" height="896" rx="218" fill="url(#base)"/>
  <rect x="64" y="64" width="896" height="896" rx="218" fill="url(#cyanGlow)"/>
  <rect x="64" y="64" width="896" height="896" rx="218" fill="url(#pinkGlow)"/>
  <path d="M148 694C270 600 330 450 454 384c126-67 245-31 421-168" fill="none" stroke="#39C5BB" stroke-width="42" stroke-linecap="round" opacity="0.22"/>
  <path d="M160 730c152-88 222-36 348-114 118-73 144-224 356-282" fill="none" stroke="url(#ribbon)" stroke-width="76" stroke-linecap="round" opacity="0.9"/>
  <path d="M308 304H202v224h106" fill="none" stroke="#39C5BB" stroke-width="58" stroke-linecap="round" stroke-linejoin="round" opacity="0.92"/>
  <path d="M716 304h106v224H716" fill="none" stroke="#E12885" stroke-width="58" stroke-linecap="round" stroke-linejoin="round" opacity="0.92"/>
  <g filter="url(#softShadow)">
    <circle cx="404" cy="694" r="88" fill="#F7FFFF"/>
    <circle cx="640" cy="646" r="82" fill="#F7FFFF"/>
    <rect x="478" y="318" width="58" height="378" rx="20" fill="#F7FFFF"/>
    <rect x="704" y="270" width="58" height="372" rx="20" fill="#F7FFFF"/>
    <path d="M507 338 733 290" fill="none" stroke="#F7FFFF" stroke-width="78" stroke-linecap="round"/>
    <circle cx="404" cy="694" r="101" fill="none" stroke="#39C5BB" stroke-width="18" opacity="0.72"/>
    <circle cx="640" cy="646" r="95" fill="none" stroke="#39C5BB" stroke-width="18" opacity="0.72"/>
    <path d="M507 338 733 290" fill="none" stroke="#39C5BB" stroke-width="96" stroke-linecap="round" opacity="0.28"/>
  </g>
  <circle cx="676" cy="612" r="24" fill="#39C5BB"/><circle cx="746" cy="612" r="24" fill="#F7FFFF" opacity="0.92"/><circle cx="816" cy="612" r="24" fill="#E12885"/>
  <path d="M304 842h126" stroke="#39C5BB" stroke-width="24" stroke-linecap="round" opacity="0.55"/>
  <path d="M466 842h96" stroke="#F7FFFF" stroke-width="24" stroke-linecap="round" opacity="0.38"/>
  <path d="M598 842h126" stroke="#E12885" stroke-width="24" stroke-linecap="round" opacity="0.55"/>
  <rect x="64" y="64" width="896" height="896" rx="218" fill="none" stroke="#ffffff" stroke-width="12" opacity="0.12"/>
</svg>
"""


def clamp(value: float) -> int:
    return max(0, min(255, int(value + 0.5)))


def blend(canvas: bytearray, x: int, y: int, color: tuple[int, int, int, int]) -> None:
    if x < 0 or y < 0 or x >= SIZE or y >= SIZE:
        return
    i = (y * SIZE + x) * 4
    sr, sg, sb, sa_i = color
    sa = sa_i / 255
    da = canvas[i + 3] / 255
    outa = sa + da * (1 - sa)
    if outa <= 0:
        return
    canvas[i] = clamp((sr * sa + canvas[i] * da * (1 - sa)) / outa)
    canvas[i + 1] = clamp((sg * sa + canvas[i + 1] * da * (1 - sa)) / outa)
    canvas[i + 2] = clamp((sb * sa + canvas[i + 2] * da * (1 - sa)) / outa)
    canvas[i + 3] = clamp(outa * 255)


def inside_round(x: float, y: float, rx: float, ry: float, w: float, h: float, r: float) -> bool:
    cx = min(max(x, rx + r), rx + w - r)
    cy = min(max(y, ry + r), ry + h - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r**2


def dist_to_segment(px: float, py: float, ax: float, ay: float, bx: float, by: float) -> float:
    vx, vy = bx - ax, by - ay
    wx, wy = px - ax, py - ay
    c = vx * vx + vy * vy
    if c == 0:
        return math.hypot(px - ax, py - ay)
    t = max(0, min(1, (wx * vx + wy * vy) / c))
    return math.hypot(px - (ax + t * vx), py - (ay + t * vy))


def draw_polyline(canvas: bytearray, points: list[tuple[float, float]], width: float, color: tuple[int, int, int, int], gradient: tuple[int, int, int, tuple[int, int, int], tuple[int, int, int]] | None = None) -> None:
    rad = width / 2
    minx = max(0, int(min(p[0] for p in points) - rad - 2))
    maxx = min(SIZE - 1, int(max(p[0] for p in points) + rad + 2))
    miny = max(0, int(min(p[1] for p in points) - rad - 2))
    maxy = min(SIZE - 1, int(max(p[1] for p in points) + rad + 2))
    for y in range(miny, maxy + 1):
        for x in range(minx, maxx + 1):
            d = min(dist_to_segment(x + 0.5, y + 0.5, *points[i], *points[i + 1]) for i in range(len(points) - 1))
            if d <= rad + 1:
                aa = max(0, min(1, rad + 1 - d))
                col = color
                if gradient is not None:
                    gx0, _, gx1, c0, c1 = gradient
                    t = max(0, min(1, (x - gx0) / (gx1 - gx0)))
                    col = (int(c0[0] * (1 - t) + c1[0] * t), int(c0[1] * (1 - t) + c1[1] * t), int(c0[2] * (1 - t) + c1[2] * t), color[3])
                blend(canvas, x, y, (col[0], col[1], col[2], int(col[3] * aa)))


def draw_circle(canvas: bytearray, cx: float, cy: float, r: float, color: tuple[int, int, int, int]) -> None:
    for y in range(max(0, int(cy - r - 2)), min(SIZE, int(cy + r + 3))):
        for x in range(max(0, int(cx - r - 2)), min(SIZE, int(cx + r + 3))):
            d = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
            if d <= r + 1:
                blend(canvas, x, y, (color[0], color[1], color[2], int(color[3] * max(0, min(1, r + 1 - d)))))


def draw_rect(canvas: bytearray, x0: int, y0: int, x1: int, y1: int, color: tuple[int, int, int, int]) -> None:
    for y in range(max(0, y0), min(SIZE, y1)):
        for x in range(max(0, x0), min(SIZE, x1)):
            blend(canvas, x, y, color)


def render() -> bytearray:
    canvas = bytearray(SIZE * SIZE * 4)
    for y in range(64, 960):
        for x in range(64, 960):
            if not inside_round(x + 0.5, y + 0.5, 64, 64, 896, 896, 218):
                continue
            t = (x + y) / (2 * SIZE)
            r, g, b = int(16 * (1 - t) + 24 * t), int(24 * (1 - t) + 17 * t), int(36 * (1 - t) + 38 * t)
            for cx, cy, radius, cr, cg, cb, strength in ((330, 270, 430, 57, 197, 187, 0.65), (750, 770, 430, 225, 40, 133, 0.58)):
                d = math.hypot(x - cx, y - cy) / radius
                if d < 1:
                    a = (1 - d) ** 2 * strength
                    r, g, b = int(r * (1 - a) + cr * a), int(g * (1 - a) + cg * a), int(b * (1 - a) + cb * a)
            blend(canvas, x, y, (r, g, b, 255))

    draw_polyline(canvas, [(148, 694), (300, 592), (454, 384), (630, 376), (875, 216)], 42, (57, 197, 187, 55))
    draw_polyline(canvas, [(160, 730), (330, 662), (508, 616), (628, 466), (864, 334)], 76, (255, 255, 255, 230), (160, 0, 864, (57, 197, 187), (225, 40, 133)))
    draw_polyline(canvas, [(308, 304), (202, 304), (202, 528), (308, 528)], 58, (57, 197, 187, 235))
    draw_polyline(canvas, [(716, 304), (822, 304), (822, 528), (716, 528)], 58, (225, 40, 133, 235))
    # 双八分音符：两个音符头、符干与横梁，保证小尺寸也能读作音乐。
    draw_circle(canvas, 404, 694, 98, (0, 0, 0, 90))
    draw_circle(canvas, 640, 646, 92, (0, 0, 0, 90))
    draw_rect(canvas, 478, 318, 536, 696, (0, 0, 0, 90))
    draw_rect(canvas, 704, 270, 762, 642, (0, 0, 0, 90))
    draw_polyline(canvas, [(507, 338), (733, 290)], 86, (0, 0, 0, 90))
    draw_circle(canvas, 404, 694, 88, (247, 255, 255, 255))
    draw_circle(canvas, 640, 646, 82, (247, 255, 255, 255))
    draw_rect(canvas, 478, 318, 536, 696, (247, 255, 255, 255))
    draw_rect(canvas, 704, 270, 762, 642, (247, 255, 255, 255))
    draw_polyline(canvas, [(507, 338), (733, 290)], 78, (247, 255, 255, 255))
    draw_circle(canvas, 404, 694, 104, (57, 197, 187, 72))
    draw_circle(canvas, 640, 646, 98, (57, 197, 187, 72))
    draw_polyline(canvas, [(507, 338), (733, 290)], 98, (57, 197, 187, 66))
    for cx, col in ((676, (57, 197, 187, 255)), (746, (247, 255, 255, 235)), (816, (225, 40, 133, 255))):
        draw_circle(canvas, cx, 612, 24, col)
    draw_polyline(canvas, [(304, 842), (430, 842)], 24, (57, 197, 187, 140))
    draw_polyline(canvas, [(466, 842), (562, 842)], 24, (247, 255, 255, 96))
    draw_polyline(canvas, [(598, 842), (724, 842)], 24, (225, 40, 133, 140))
    return canvas


def png_bytes(width: int, height: int, data: bytes) -> bytes:
    raw = bytearray()
    stride = width * 4
    for y in range(height):
        raw.append(0)
        raw.extend(data[y * stride : (y + 1) * stride])

    def chunk(kind: bytes, payload: bytes) -> bytes:
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)

    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")


def downsample(data: bytes, size: int) -> bytearray:
    scale = SIZE // size
    out = bytearray(size * size * 4)
    for y in range(size):
        for x in range(size):
            totals = [0, 0, 0, 0]
            for yy in range(y * scale, (y + 1) * scale):
                base = (yy * SIZE + x * scale) * 4
                for xx in range(scale):
                    i = base + xx * 4
                    for channel in range(4):
                        totals[channel] += data[i + channel]
            o = (y * size + x) * 4
            count = scale * scale
            for channel in range(4):
                out[o + channel] = totals[channel] // count
    return out


def bmp_dib(size: int, data: bytes) -> bytes:
    header = struct.pack("<IIIHHIIIIII", 40, size, size * 2, 1, 32, 0, size * size * 4, 0, 0, 0, 0)
    pixels = bytearray()
    for y in range(size - 1, -1, -1):
        for x in range(size):
            i = (y * size + x) * 4
            pixels.extend((data[i + 2], data[i + 1], data[i], data[i + 3]))
    mask_stride = ((size + 31) // 32) * 4
    return header + pixels + bytes(mask_stride * size)


def write_ico(data: bytes) -> None:
    images = []
    for size in (256, 128, 64, 48, 32, 16):
        images.append((size, bmp_dib(size, downsample(data, size))))
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    directory = bytearray()
    body = bytearray()
    for size, payload in images:
        directory.extend(struct.pack("<BBBBHHII", 0 if size == 256 else size, 0 if size == 256 else size, 0, 0, 1, 32, len(payload), offset))
        body.extend(payload)
        offset += len(payload)
    (OUT / "icon.ico").write_bytes(header + directory + body)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "icon.svg").write_text(SVG, encoding="utf-8")
    raster = render()
    (OUT / "icon.png").write_bytes(png_bytes(SIZE, SIZE, raster))
    write_ico(raster)
    print("generated src-tauri/icons/icon.svg, icon.png, icon.ico")


if __name__ == "__main__":
    main()
