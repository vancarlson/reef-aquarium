"""Per-vertex body thickness for each fish cutout, from its alpha silhouette.
Writes assets/fish/<name>.thick: (NY+1) rows x (NX+1) cols of u8 (0..255)."""
import sys, glob, os
import numpy as np, cv2
from PIL import Image
NX, NY = 72, 28
for f in sorted(glob.glob('assets/fish/*.png')):
    a = np.array(Image.open(f).convert('RGBA'))[..., 3]
    h, w = a.shape
    s = 256 / max(h, w)
    a = cv2.resize(a, (max(1, int(w * s)), max(1, int(h * s))), interpolation=cv2.INTER_AREA)
    m = (a > 127).astype(np.uint8)
    m = np.pad(m, 4)
    d = cv2.distanceTransform(m, cv2.DIST_L2, 5)[4:-4, 4:-4]
    d = cv2.GaussianBlur(d, (0, 0), 2.0)
    d = np.sqrt(d / max(d.max(), 1e-6))          # rounded cross-section
    d[a < 127] = 0
    g = cv2.resize(d, (NX + 1, NY + 1), interpolation=cv2.INTER_LINEAR)
    g = np.clip(g, 0, 1)
    out = os.path.splitext(f)[0] + '.thick'
    open(out, 'wb').write((g * 255).astype(np.uint8).tobytes())
    print(out, g.max().round(2))
