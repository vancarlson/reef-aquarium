#!/usr/bin/env python3
"""Cut extra rock pieces out of the live-rock cutout (assets_src/reef/rock_a.png) so the tank is not
built from copies of one rock. Each piece is a different part of the photo with an irregular, feathered edge."""
import cv2, numpy as np
rng = np.random.default_rng(11)
src = cv2.imread("assets_src/reef/rock_a.png", cv2.IMREAD_UNCHANGED)
H, W = src.shape[:2]
def piece(name, x0, y0, x1, y1, outw, darken=1.0, seed=0):
    p = src[y0:y1, x0:x1].copy()
    h, w = p.shape[:2]
    r = np.random.default_rng(seed)
    n = r.random((h // 40 + 2, w // 40 + 2)).astype(np.float32)
    n = cv2.resize(n, (w, h), interpolation=cv2.INTER_CUBIC)
    n = cv2.GaussianBlur(n, (0, 0), 25)
    n = (n - n.min()) / (n.max() - n.min() + 1e-6) - 0.5
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    # distance to the crop edge, as a fraction of the size, wobbled by noise -> irregular broken edge
    d = np.minimum.reduce([xx / w, (w - xx) / w, yy / h, (h - yy) / h])
    m = np.clip((d + 0.16 * n - 0.06) / 0.06, 0, 1)
    a = p[..., 3].astype(np.float32) / 255.0 * m
    # keep one cohesive piece: largest component, holes filled, edge softened
    b = (a > 0.45).astype(np.uint8)
    cnt, lab, st, _ = cv2.connectedComponentsWithStats(b)
    if cnt > 1:
        b = (lab == 1 + np.argmax(st[1:, cv2.CC_STAT_AREA])).astype(np.uint8)
    cs, _ = cv2.findContours(b, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    b = np.zeros_like(b); cv2.drawContours(b, cs, -1, 1, -1)
    a = cv2.GaussianBlur(b.astype(np.float32), (0, 0), 3.0)
    p[..., :3] = np.clip(p[..., :3].astype(np.float32) * darken, 0, 255).astype(np.uint8)
    p[..., 3] = (a * 255).astype(np.uint8)
    ys, xs = np.where(p[..., 3] > 8)
    p = p[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    s = outw / p.shape[1]
    p = cv2.resize(p, (outw, int(p.shape[0] * s)), interpolation=cv2.INTER_AREA)
    cv2.imwrite(f"assets/rock/{name}.png", p)
    print(name, p.shape[1], p.shape[0], "aspect %.3f" % (p.shape[1] / p.shape[0]))
piece("rock_b", 230, 380, 1330, 1650, 900, 1.0, 1)   # left boulder
piece("rock_c", 1250, 250, 2290, 1500, 900, 0.95, 2) # right-hand ledge
piece("rock_d", 700, 120, 1700, 1000, 800, 0.85, 3)  # upper middle, darker
