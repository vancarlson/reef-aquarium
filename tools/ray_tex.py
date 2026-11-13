#!/usr/bin/env python3
"""Generate assets/tex/ray.png: a soft shaft of sunlight. Fades in just below the surface, peaks, then
fades slowly with depth; wide soft core with a few faint streaks; no hard edges anywhere."""
import numpy as np
from PIL import Image
W, H = 128, 512
u = np.linspace(0, 1, W)[None, :].repeat(H, 0)
v = np.linspace(0, 1, H)[:, None].repeat(W, 1)
g = lambda c, s: np.exp(-((u - c) / s) ** 2)
prof = 0.75 * g(0.5, 0.20) + 0.30 * g(0.36, 0.045) + 0.24 * g(0.63, 0.04) + 0.14 * g(0.5, 0.40)
prof = prof * (0.86 + 0.14 * np.sin(v * 11 + u * 5))
ss = lambda a, b, x: np.clip((x - a) / (b - a), 0, 1) ** 2 * (3 - 2 * np.clip((x - a) / (b - a), 0, 1))
vert = ss(0.0, 0.18, v) * (1 - ss(0.25, 1.0, v)) ** 1.4
edge = ss(0.0, 0.12, u) * ss(0.0, 0.12, 1 - u)
img = np.clip(prof * vert * edge, 0, 1)
img = (img / img.max() * 255).astype(np.uint8)
Image.fromarray(np.dstack([img, img, img])).save("assets/tex/ray.png")
print("ray.png ok")
