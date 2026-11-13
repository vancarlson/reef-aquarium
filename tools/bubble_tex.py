#!/usr/bin/env python3
"""Generate assets/tex/bubble.png: an underwater air bubble (transparent core, bright Fresnel rim,
window reflections, dark thin outline). Original procedural texture."""
import numpy as np
from PIL import Image
N = 192
y, x = np.mgrid[0:N, 0:N].astype(np.float32)
x = (x - N / 2 + 0.5) / (N / 2); y = (y - N / 2 + 0.5) / (N / 2)
r = np.hypot(x, y)
ss = lambda a, b, v: np.clip((v - a) / (b - a), 0, 1) ** 2 * (3 - 2 * np.clip((v - a) / (b - a), 0, 1))
disc = 1 - ss(0.97, 1.0, r)
# Fresnel-like rim: nearly clear in the middle, brighter toward the edge
rim = ss(0.55, 0.96, r) ** 1.6
# light from upper left: rim is brighter on the lit side and on the opposite side (internal reflection)
ang = np.arctan2(y, x)
lit = 0.55 + 0.45 * np.cos(ang - np.deg2rad(-135)) ** 2
col = np.array([0.80, 0.93, 1.0])
alpha = disc * (0.05 + 0.55 * rim * lit)
# soft body tint, very faint
alpha += disc * 0.04
# big window highlight (upper left)
hx, hy = -0.38, -0.42
hl = np.exp(-(((x - hx) / 0.20) ** 2 + ((y - hy) / 0.11) ** 2)) * disc
# small secondary highlight (lower right, through the bubble)
hl2 = np.exp(-(((x - 0.42) / 0.10) ** 2 + ((y - 0.40) / 0.07) ** 2)) * disc * 0.55
bright = np.clip(hl + hl2, 0, 1)
# thin darker outline just inside the edge
outline = ss(0.90, 0.95, r) * (1 - ss(0.95, 0.985, r))
rgb = np.zeros((N, N, 3), np.float32)
rgb[:] = col
rgb = rgb * (1 - 0.35 * outline[..., None])
rgb = rgb * (1 - bright[..., None]) + bright[..., None]
alpha = np.clip(alpha + bright * 0.95, 0, 1)
out = np.dstack([rgb, alpha])
Image.fromarray((np.clip(out, 0, 1) * 255).astype(np.uint8), "RGBA").save("assets/tex/bubble.png")
print("bubble.png ok")
