#!/usr/bin/env python3
"""Cut the whitetip reef shark (Commons: Whitetip-reef-shark.jpg, public domain, Jan Derk) out of the photo.
GrabCut on the clean blue water, remove the blue colour cast, level the body, face right -> assets/fish/shark.png"""
import cv2, numpy as np, sys
src = sys.argv[1] if len(sys.argv) > 1 else "assets_src/shark/g7.jpg"
img = cv2.imread(src)
x0, y0, x1, y1 = 520, 395, 1545, 880
mask = np.zeros(img.shape[:2], np.uint8)
bgd, fgd = np.zeros((1, 65)), np.zeros((1, 65))
cv2.grabCut(img, mask, (x0, y0, x1 - x0, y1 - y0), bgd, fgd, 10, cv2.GC_INIT_WITH_RECT)
m = np.where((mask == 1) | (mask == 3), 255, 0).astype(np.uint8)
n, lab, st, _ = cv2.connectedComponentsWithStats(m)
m = np.where(lab == 1 + np.argmax(st[1:, cv2.CC_STAT_AREA]), 255, 0).astype(np.uint8)
k = lambda r: cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (2*r+1, 2*r+1))
m = cv2.morphologyEx(m, cv2.MORPH_CLOSE, k(4))
# rough outline (source pixels) keeps GrabCut from grabbing the reef water below the shark
poly = np.array([(538,825),(590,760),(700,695),(800,650),(885,575),(915,500),(938,438),(970,520),(1010,533),(1110,515),(1145,470),(1185,442),(1200,500),(1225,518),
 (1290,520),(1400,455),(1470,425),(1527,413),(1500,445),(1420,505),(1370,545),(1405,635),(1340,605),(1285,572),(1215,575),(1212,635),(1170,610),(1112,600),
 (1105,665),(1060,680),(1035,650),(1030,700),(1000,740),(1030,835),(900,775),(880,790),(760,835),(660,858),(570,848)], np.int32)
rough = np.zeros(m.shape, np.uint8); cv2.fillPoly(rough, [poly], 255)
cv2.imwrite("assets_src/shark/g7_rough.png", rough)
red = cv2.GaussianBlur(img[..., 2], (0, 0), 1.0)   # water has almost no red, the shark does
m = cv2.bitwise_and(m, cv2.dilate(rough, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (31, 31))))
m = cv2.bitwise_and(m, np.where(red > 30, 255, 0).astype(np.uint8))
m = cv2.morphologyEx(m, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
n, lab, st, _ = cv2.connectedComponentsWithStats(m)
m = np.where(lab == 1 + np.argmax(st[1:, cv2.CC_STAT_AREA]), 255, 0).astype(np.uint8)
# take the fringe away: shrink the matte a hair, soften it, and pull the edge colour inward
m = cv2.erode(m, k(1))
a = cv2.GaussianBlur(m, (0, 0), 1.1)
# water colour cast out: the photo is very blue; balance towards a neutral grey-brown body, a little more contrast
f = img.astype(np.float32)
f[..., 0] *= 0.55; f[..., 1] *= 0.62; f[..., 2] *= 1.30
f = np.clip(f, 0, 255)
f = np.clip((f - 128) * 1.15 + 120, 0, 255)
rgb = f.astype(np.uint8)
# replace the edge pixels' colour by the nearest interior colour (no blue halo)
core = cv2.erode(m, k(4))
dist_src = np.where(core > 0, 0, 1).astype(np.uint8)
_, labels = cv2.distanceTransformWithLabels(dist_src, cv2.DIST_L2, 5, labelType=cv2.DIST_LABEL_PIXEL)
ys, xs = np.where(core > 0)
lut = {}
idx = labels[ys, xs]
colors = rgb[ys, xs]
table = np.zeros((labels.max() + 1, 3), np.uint8); table[idx] = colors
edge = (m > 0) & (core == 0)
rgb2 = rgb.copy(); rgb2[edge] = table[labels[edge]]
# level the body axis (snout -> tail root), face right
ang = float(np.degrees(np.arctan2(820 - 525, 1250 - 545)))
big = cv2.copyMakeBorder(np.dstack([rgb2, a]), 250, 250, 250, 250, cv2.BORDER_CONSTANT, value=(0, 0, 0, 0))
R = cv2.getRotationMatrix2D((big.shape[1] / 2, big.shape[0] / 2), -ang, 1.0)
big = cv2.warpAffine(big, R, (big.shape[1], big.shape[0]), flags=cv2.INTER_CUBIC)
ys, xs = np.where(big[..., 3] > 8)
out = big[ys.min():ys.max()+1, xs.min():xs.max()+1][:, ::-1]
s = 900 / out.shape[1]
out = cv2.resize(out, (900, int(out.shape[0] * s)), interpolation=cv2.INTER_AREA)
cv2.imwrite("assets/fish/shark.png", out)
print("shark aspect %.4f" % (out.shape[1] / out.shape[0]), out.shape)
bg = np.full(out.shape[:2] + (3,), (150, 90, 30), np.uint8); al = out[..., 3:] / 255.
cv2.imwrite("/tmp/claude-1000/-home-paul-code-fish/e39fe1f7-665c-460a-872c-37fe4fa1c05d/scratchpad/shark_prev.png", (out[..., :3] * al + bg * (1 - al)).astype(np.uint8))
