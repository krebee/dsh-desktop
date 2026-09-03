#!/usr/bin/env python3
"""Generate the dsh-desktop app icon (1024x1024 PNG).

Usage: python3 scripts/make-icon.py   -> writes app-icon.png at project root
"""
import os

from PIL import Image, ImageDraw, ImageFont

SIZE = 1024
RADIUS = 224
TOP = (79, 140, 255)      # #4F8CFF
BOTTOM = (124, 92, 255)   # #7C5CFF
OUT = os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "app-icon.png")
)


def main() -> None:
    # Gradient background (per-row colors) clipped to a rounded-square mask.
    gradient = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    px = gradient.load()
    for y in range(SIZE):
        t = y / (SIZE - 1)
        color = (int(TOP[i] + (BOTTOM[i] - TOP[i]) * t) for i in range(3))
        r, g, b = color
        for x in range(SIZE):
            px[x, y] = (r, g, b, 255)

    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, SIZE - 1, SIZE - 1], radius=RADIUS, fill=255
    )
    gradient.putalpha(mask)

    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    img.paste(gradient, (0, 0), gradient)
    draw = ImageDraw.Draw(img)

    # "DSH" monogram, centered.
    font_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    ]
    font = None
    for path in font_paths:
        if os.path.exists(path):
            font = ImageFont.truetype(path, 430)
            break
    if font is None:
        font = ImageFont.load_default()

    text = "DSH"
    bbox = draw.textbbox((0, 0), text, font=font)
    tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
    x = (SIZE - tw) // 2 - bbox[0]
    y = (SIZE - th) // 2 - bbox[1]
    draw.text((x, y), text, font=font, fill=(255, 255, 255, 255))

    img.save(OUT, "PNG")
    print(f"wrote {OUT} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()