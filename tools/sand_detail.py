#!/usr/bin/env python3
"""Make assets/tex/sand_detail.png: a neutral, tileable grain/pebble layer from the CC0 Poly Haven
'Coral Gravel' diffuse map. Only the fine light/dark detail is kept (colour removed), so it can be laid
over the pale sand to add real grain without turning it brown."""
import cv2, numpy as np
img = cv2.imread("assets_src/tex/coral_gravel_Diffuse.jpg")
img = cv2.resize(img, (1024, 1024), interpolation=cv2.INTER_AREA)
L = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY).astype(np.float32) / 255.0
# wrap-around blur so the result still tiles
big = np.tile(L, (3, 3))
low = cv2.GaussianBlur(big, (0, 0), 9)[1024:2048, 1024:2048]
hp = L - low
hp = hp / (np.percentile(np.abs(hp), 99) + 1e-6)
light = np.clip(hp, 0, 1)
dark = np.clip(-hp, 0, 1)
a = np.clip(0.55 * np.maximum(light, dark), 0, 0.6)
col = np.where((hp > 0)[..., None], np.array([255, 250, 238.]), np.array([70, 62, 52.]))
rgba = np.dstack([col[..., 2], col[..., 1], col[..., 0], a * 255]).astype(np.uint8)  # to BGRA for cv2
cv2.imwrite("assets/tex/sand_detail.png", rgba)
print("ok", rgba.shape)
