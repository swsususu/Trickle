#!/usr/bin/env python3
"""Generates the Trickle app icon source image.

The mark is a droplet (trickle charge, which the name comes from) whose
interior carries a bolt cut out of it. Drawn at 4x and downsampled so the
curves and the bolt edges stay clean without a rasteriser.

Usage: python3 scripts/generate_icon.py
Writes app-icon.png at the repo root, which `pnpm tauri icon` consumes.
"""

from PIL import Image, ImageDraw
import math
import os

SIZE = 1024
SS = 4  # supersampling factor
W = SIZE * SS

# macOS app icons are not full-bleed: the artwork sits in roughly the middle
# 80% so it optically matches system icons in the Dock.
MARGIN = int(W * 0.105)
BOX = W - MARGIN * 2

# Blue, matching the accent already used in the UI (tailwind blue-500/600).
TOP = (96, 165, 250)
BOTTOM = (29, 78, 216)


def rounded_rect_mask(size: int, radius: int) -> Image.Image:
    """macOS squircle approximation via a large-radius rounded rectangle."""
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return mask


def vertical_gradient(size: int, top: tuple, bottom: tuple) -> Image.Image:
    """A vertical gradient, drawn one row at a time."""
    grad = Image.new("RGB", (1, size))
    px = grad.load()
    for y in range(size):
        t = y / (size - 1)
        px[0, y] = tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
    return grad.resize((size, size), Image.BILINEAR)


def droplet_points(cx: float, cy: float, w: float, h: float, steps: int = 720):
    """Droplet outline: a pointed apex easing into a circular base.

    The horizontal half-width follows sin(t)^0.72 as t runs from the apex to
    the base. The exponent below 1 keeps the shoulders full instead of letting
    them collapse into a narrow cone.
    """
    pts = []
    for i in range(steps + 1):
        t = math.pi * i / steps
        y = cy - h / 2 + h * (i / steps)
        half = (w / 2) * math.sin(t) ** 0.72
        pts.append((cx - half, y))
    for i in range(steps, -1, -1):
        t = math.pi * i / steps
        y = cy - h / 2 + h * (i / steps)
        half = (w / 2) * math.sin(t) ** 0.72
        pts.append((cx + half, y))
    return pts


def bolt_points(cx: float, cy: float, w: float, h: float):
    """A lightning bolt, described in a 0..1 box and mapped onto the target."""
    raw = [
        (0.60, 0.00),
        (0.14, 0.56),
        (0.44, 0.56),
        (0.36, 1.00),
        (0.86, 0.40),
        (0.55, 0.40),
    ]
    return [
        (cx - w / 2 + x * w, cy - h / 2 + y * h)
        for x, y in raw
    ]


def main() -> None:
    canvas = Image.new("RGBA", (W, W), (0, 0, 0, 0))

    # Squircle background with the gradient clipped into it.
    grad = vertical_gradient(BOX, TOP, BOTTOM).convert("RGBA")
    mask = rounded_rect_mask(BOX, radius=int(BOX * 0.235))
    canvas.paste(grad, (MARGIN, MARGIN), mask)

    layer = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    # Droplet. The shape is narrow at the apex and wide at the base, so its
    # optical centre sits below its geometric centre; the box is nudged up to
    # compensate and leave even margins top and bottom.
    drop_w = BOX * 0.52
    drop_h = BOX * 0.64
    drop_cx = W / 2
    drop_cy = MARGIN + BOX * 0.505
    d.polygon(droplet_points(drop_cx, drop_cy, drop_w, drop_h), fill=(255, 255, 255, 255))

    # Bolt knocked out of the droplet, nudged just below the droplet's centre.
    # The upper half tapers to the apex, so a pointed shape reads better with
    # slightly more clearance above it than below; measured white margins are
    # 156px above and 135px below at 1024px.
    bolt_h = drop_h * 0.44
    bolt_w = bolt_h * 0.60
    d.polygon(
        bolt_points(drop_cx, drop_cy + drop_h * 0.02, bolt_w, bolt_h),
        fill=(0, 0, 0, 0),
    )

    canvas.alpha_composite(layer)

    out = canvas.resize((SIZE, SIZE), Image.LANCZOS)
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
    path = os.path.join(root, "app-icon.png")
    out.save(path)
    print(f"wrote {os.path.relpath(path, root)} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
