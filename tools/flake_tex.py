#!/usr/bin/env python3
"""Procedural fish-food flake texture (original): an irregular, slightly translucent flake with mottled warm colour. -> assets/tex/flake.png"""
import numpy as np, cv2
rng = np.random.default_rng(7)
S = 128
img = np.zeros((S, S, 4), np.float32)
n = 11
ang = np.sort(rng.uniform(0, 2*np.pi, n))
rad = rng.uniform(0.55, 0.92, n) * S / 2
pts = np.array([(S/2 + r*np.cos(a), S/2 + r*0.8*np.sin(a)) for a, r in zip(ang, rad)], np.float32)
mask = np.zeros((S, S), np.uint8); cv2.fillPoly(mask, [pts.astype(np.int32)], 255)
mask = cv2.GaussianBlur(mask, (0, 0), 2.2).astype(np.float32) / 255
noise = cv2.GaussianBlur(rng.random((S, S)).astype(np.float32), (0, 0), 5); noise = (noise - noise.min()) / (noise.max() - noise.min())
fine = cv2.GaussianBlur(rng.random((S, S)).astype(np.float32), (0, 0), 1.2); fine = (fine - fine.min()) / (fine.max() - fine.min())
base = np.array([0.30, 0.52, 0.93]); red = np.array([0.16, 0.22, 0.78]); gold = np.array([0.35, 0.72, 0.98])  # BGR
col = base[None, None] * (1 - noise[..., None]) + red[None, None] * noise[..., None]
col = col * (0.85 + 0.3 * fine[..., None]) + gold[None, None] * (fine[..., None] > 0.75) * 0.25
edge = cv2.GaussianBlur(mask, (0, 0), 6); col = col * (0.75 + 0.35 * edge[..., None])   # darker rim, lit middle
img[..., :3] = np.clip(col, 0, 1)
img[..., 3] = mask * 0.95
cv2.imwrite("assets/tex/flake.png", (img * 255).astype(np.uint8))
