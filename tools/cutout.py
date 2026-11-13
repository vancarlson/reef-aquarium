#!/usr/bin/env python3
"""GrabCut a fish out of a photo -> RGBA PNG cropped to the fish.
usage: cutout.py in.jpg out.png x0 y0 x1 y1   (rect in source pixels around the fish)"""
import sys, cv2, numpy as np
src, dst = sys.argv[1], sys.argv[2]
x0, y0, x1, y1 = map(int, sys.argv[3:7])
close_frac = float(sys.argv[7]) if len(sys.argv) > 7 else 0.0  # fill concave gaps (fraction of fish width)
img = cv2.imread(src)
s = 1400 / max(img.shape[:2]) if max(img.shape[:2]) > 1400 else 1.0   # work at reduced size
small = cv2.resize(img, None, fx=s, fy=s, interpolation=cv2.INTER_AREA)
r = tuple(int(v * s) for v in (x0, y0, x1 - x0, y1 - y0))
mask = np.zeros(small.shape[:2], np.uint8)
bgd, fgd = np.zeros((1, 65)), np.zeros((1, 65))
cv2.grabCut(small, mask, r, bgd, fgd, 8, cv2.GC_INIT_WITH_RECT)
m = np.where((mask == 1) | (mask == 3), 255, 0).astype(np.uint8)
# keep largest component, fill holes, smooth edge
n, lab, st, _ = cv2.connectedComponentsWithStats(m)
if n > 1:
    m = np.where(lab == 1 + np.argmax(st[1:, cv2.CC_STAT_AREA]), 255, 0).astype(np.uint8)
if close_frac > 0:
    k = max(3, int(close_frac * (x1 - x0) * s)) | 1
    m = cv2.morphologyEx(m, cv2.MORPH_CLOSE, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (k, k)))
cnts, _ = cv2.findContours(m, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
m = np.zeros_like(m); cv2.drawContours(m, cnts, -1, 255, -1)
m = cv2.GaussianBlur(m, (0, 0), 1.2)
m = cv2.resize(m, (img.shape[1], img.shape[0]), interpolation=cv2.INTER_CUBIC)
rgba = np.dstack([img, m])
ys, xs = np.where(m > 8)
rgba = rgba[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
cv2.imwrite(dst, rgba)
print("saved", dst, rgba.shape)
