#!/usr/bin/env python3
"""Generate the S3 Browser Tool app icon (1024x1024 PNG).

Draws a macOS-style rounded-square icon: dark gradient background
with an S3 bucket glyph and the app initials.
"""

from PIL import Image, ImageDraw, ImageFont
import os

SIZE = 1024
INSET = 100          # margin around the rounded square (Apple icon grid)
RADIUS = 185         # corner radius of the rounded square

img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# --- Background: vertical gradient inside a rounded square ---
top = (38, 44, 84)       # dark indigo
bottom = (16, 18, 36)    # near-black navy
grad = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
gdraw = ImageDraw.Draw(grad)
for y in range(SIZE):
    t = y / SIZE
    r = int(top[0] + (bottom[0] - top[0]) * t)
    g = int(top[1] + (bottom[1] - top[1]) * t)
    b = int(top[2] + (bottom[2] - top[2]) * t)
    gdraw.line([(0, y), (SIZE, y)], fill=(r, g, b, 255))

mask = Image.new("L", (SIZE, SIZE), 0)
mdraw = ImageDraw.Draw(mask)
mdraw.rounded_rectangle(
    [INSET, INSET, SIZE - INSET, SIZE - INSET], radius=RADIUS, fill=255
)
img.paste(grad, (0, 0), mask)

# --- Bucket glyph (cylinder with open top) ---
accent = (96, 165, 250)      # light blue
accent_dim = (59, 108, 180)

cx = SIZE // 2
bucket_top = 300
bucket_bottom = 640
top_w = 380       # half-width at the rim
bottom_w = 300    # half-width at the base
ellipse_h = 70

# Body (trapezoid)
draw.polygon(
    [
        (cx - top_w, bucket_top),
        (cx + top_w, bucket_top),
        (cx + bottom_w, bucket_bottom),
        (cx - bottom_w, bucket_bottom),
    ],
    fill=accent_dim,
)
# Base ellipse
draw.ellipse(
    [cx - bottom_w, bucket_bottom - ellipse_h // 2,
     cx + bottom_w, bucket_bottom + ellipse_h // 2],
    fill=accent_dim,
)
# Rim ellipse (open top)
draw.ellipse(
    [cx - top_w, bucket_top - ellipse_h,
     cx + top_w, bucket_top + ellipse_h],
    fill=accent,
)
draw.ellipse(
    [cx - top_w + 36, bucket_top - ellipse_h + 26,
     cx + top_w - 36, bucket_top + ellipse_h - 26],
    fill=(22, 26, 50),
)

# --- Label ---
font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 220, index=1)  # bold
label = "S3"
bbox = draw.textbbox((0, 0), label, font=font)
tw = bbox[2] - bbox[0]
th = bbox[3] - bbox[1]
draw.text(
    (cx - tw / 2 - bbox[0], 690 + (160 - th) / 2 - bbox[1]),
    label,
    font=font,
    fill=(235, 240, 255),
)

out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icon_1024.png")
img.save(out)
print(f"wrote {out}")
