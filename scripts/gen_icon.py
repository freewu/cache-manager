#!/usr/bin/env python3
"""Generate a simple app icon (1024x1024 PNG) - database cylinder motif."""
import struct, zlib, math

SIZE = 1024

def px(x, y):
    """Return (r,g,b,a) for pixel at (x,y)."""
    # rounded-rect mask
    margin = 64
    radius = 180
    inside = True
    # rounded rect test
    if x < margin + radius and y < margin + radius:
        inside = (x - (margin + radius)) ** 2 + (y - (margin + radius)) ** 2 <= radius ** 2
    elif x >= SIZE - margin - radius and y < margin + radius:
        inside = (x - (SIZE - margin - radius)) ** 2 + (y - (margin + radius)) ** 2 <= radius ** 2
    elif x < margin + radius and y >= SIZE - margin - radius:
        inside = (x - (margin + radius)) ** 2 + (y - (SIZE - margin - radius)) ** 2 <= radius ** 2
    elif x >= SIZE - margin - radius and y >= SIZE - margin - radius:
        inside = (x - (SIZE - margin - radius)) ** 2 + (y - (SIZE - margin - radius)) ** 2 <= radius ** 2
    elif not (margin <= x < SIZE - margin and margin <= y < SIZE - margin):
        inside = False

    if not inside:
        return (0, 0, 0, 0)

    # background gradient (deep blue)
    t = y / SIZE
    bg = (13 + int(20 * t), 38 + int(30 * t), 90 + int(40 * t))

    # draw a ">"-like chevron + bars motif in white/light blue
    cx = SIZE // 2
    cy = SIZE // 2
    # three horizontal rounded bars, each 56 px tall
    bar_h = 56
    gap = 44
    total = 3 * bar_h + 2 * gap
    top = cy - total // 2
    bar_w_start = 300
    bar_w_end = 724
    # increasing width for the middle bar (staircase look)
    widths = [bar_w_start, bar_w_start + 70, bar_w_start + 140]
    colors = [(255, 255, 255), (220, 235, 255), (180, 210, 255)]

    for i in range(3):
        y0 = top + i * (bar_h + gap)
        w = widths[i]
        x0 = cx - w // 2
        x1 = cx + w // 2
        if y0 <= y < y0 + bar_h:
            # rounded ends
            r = bar_h // 2
            for xx in range(x0, x1):
                if xx < x0 + r:
                    if (xx - (x0 + r)) ** 2 + (y - (y0 + r)) ** 2 > r * r:
                        continue
                if xx >= x1 - r:
                    if (xx - (x1 - r - 1)) ** 2 + (y - (y0 + r)) ** 2 > r * r:
                        continue
                return colors[i] + (255,)
    return bg + (255,)

rows = []
for y in range(SIZE):
    row = bytearray()
    for x in range(SIZE):
        r, g, b, a = px(x, y)
        row += bytes((r, g, b, a))
    rows.append(bytes(row))

def chunk(tag, data):
    c = struct.pack(">I", len(data)) + tag + data
    c += struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    return c

raw = b"".join(b"\x00" + r for r in rows)
png = b"\x89PNG\r\n\x1a\n"
png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(raw, 9))
png += chunk(b"IEND", b"")

with open("app-icon.png", "wb") as f:
    f.write(png)
print("written app-icon.png", len(png), "bytes")
