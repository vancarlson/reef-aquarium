#!/usr/bin/env python3
"""Finish the new fish cutouts: trim leftover background, face right, tidy the alpha, write assets/fish/<name>.png."""
import cv2, numpy as np
def finish(src, name, flip, trim=None, width=640):
    im = cv2.imread(f"assets_src/new/{src}_cut.png", cv2.IMREAD_UNCHANGED)
    a = im[..., 3].copy()
    h, w = a.shape
    if trim is not None:
        trim(a)
    n, lab, st, _ = cv2.connectedComponentsWithStats((a > 60).astype(np.uint8))
    if n > 1:
        keep = 1 + np.argmax(st[1:, cv2.CC_STAT_AREA]); a[lab != keep] = 0
    a = cv2.GaussianBlur(a, (0, 0), 1.0)
    im[..., 3] = a
    ys, xs = np.where(a > 8)
    im = im[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    if flip: im = im[:, ::-1]
    s = width / im.shape[1]
    im = cv2.resize(im, (width, int(im.shape[0] * s)), interpolation=cv2.INTER_AREA)
    cv2.imwrite(f"assets/fish/{name}.png", im)
    print(name, im.shape[1], im.shape[0], "aspect %.4f" % (im.shape[1] / im.shape[0]))
def trim_firefish(a):
    h, w = a.shape
    im = cv2.imread('assets_src/new/firefish_cut.png')
    hsv = cv2.cvtColor(im, cv2.COLOR_BGR2HSV)
    dull = (hsv[..., 1] < 110) & (hsv[..., 2] < 150)
    dull[:, : int(0.2 * w)] = False   # keep the dark eye
    a[dull] = 0
    for x in range(658, w):
        y = int(142 + 0.3546 * (x - 658)) - 4
        a[:max(y, 0), x] = 0
finish("bicolor", "bicolor", flip=False)
finish("firefish", "firefish", flip=True, trim=trim_firefish)
finish("m3", "mandarin", flip=True)
