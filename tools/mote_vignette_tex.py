#!/usr/bin/env python3
"""Generate assets/tex/mote.png (soft plankton speck)."""
import numpy as np
from PIL import Image
N = 64
y, x = np.mgrid[0:N, 0:N].astype(np.float32); x = (x - N/2 + .5) / (N/2); y = (y - N/2 + .5) / (N/2)
r = np.hypot(x, y)
a = np.clip(1 - r, 0, 1) ** 2.2
rgba = np.dstack([np.full((N, N), 235), np.full((N, N), 245), np.full((N, N), 255), a * 255]).astype(np.uint8)
Image.fromarray(rgba, "RGBA").save("assets/tex/mote.png")
print("ok")
