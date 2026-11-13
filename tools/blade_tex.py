#!/usr/bin/env python3
"""Generate assets/tex/blade.png: a seagrass-blade texture (u across the blade, v root->tip).
Original procedural texture: fine parallel veins, darker edges, soft mottling."""
import numpy as np
from PIL import Image
W, H = 64, 512
rng = np.random.default_rng(7)
u = np.linspace(0, 1, W)[None, :].repeat(H, 0)
v = np.linspace(0, 1, H)[:, None].repeat(W, 1)
def noise(sx, sy):
    g = rng.random((sy, sx)).astype(np.float32)
    return np.array(Image.fromarray((g * 255).astype(np.uint8)).resize((W, H), Image.BICUBIC)) / 255.0
root = np.array([20, 42, 22.]); mid = np.array([58, 112, 44.]); tip = np.array([128, 158, 66.])
t = v[..., None]
col = np.where(t < 0.5, root + (mid - root) * (t / 0.5), mid + (tip - mid) * ((t - 0.5) / 0.5))
# veins: ~9 fine lines across, lighter, wobbling slightly
wob = 0.012 * np.sin(v * 25 + u * 3)
veins = 0.5 + 0.5 * np.cos((u + wob) * 2 * np.pi * 4.5)
shade = 1.0 + 0.16 * (veins - 0.5) * 2
mid_rib = np.exp(-((u - 0.5) / 0.05) ** 2)
shade += 0.18 * mid_rib
edge = np.abs(u - 0.5) * 2
shade *= 1.0 - 0.35 * edge ** 3                     # darker, thinner edges
shade *= 0.88 + 0.24 * noise(4, 24)                 # soft mottling along the blade
shade *= 0.94 + 0.12 * noise(16, 90)                # fine grain
img = np.clip(col * shade[..., None], 0, 255).astype(np.uint8)
Image.fromarray(img).save("assets/tex/blade.png")
print("blade.png", img.shape)
