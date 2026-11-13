#!/usr/bin/env python3
"""Ask Wikimedia Commons for the licence and author of every photo the aquarium ships, so CREDITS.md
can be checked against the source (run before releasing)."""
import json, re, time, urllib.parse, urllib.request
FILES = [
 "Paracanthurus-hepatus-paletten-doktorfisch.jpg", "Zebrasoma flavescens 004a.jpg", "Amphiprion percula 1.jpg",
 "Gramma loreto, Fairy Basslet.jpg", "Flame Angelfish Centropyge loricula.jpg", "Green.chromis.arp.jpg",
 "Centropyge bicolor 1.jpg", "Nemateleotris magnifica Prague 2011 1.jpg",
 "Mandarin fish (Synchiropus splendidus) (45103629692).jpg",
 "Whitetip-reef-shark.jpg",
 "Coralline algae encrusted live rock.jpg", "Caulastraea y Zoanthus.jpg", "Zoanthus (pseudo Anêmona).jpg",
]
for f in FILES:
    q = urllib.parse.urlencode({"action": "query", "titles": "File:" + f, "prop": "imageinfo", "iiprop": "extmetadata|url",
                                "format": "json", "formatversion": "2"})
    req = urllib.request.Request("https://commons.wikimedia.org/w/api.php?" + q, headers={"User-Agent": "reef-aquarium-hobby/1.0"})
    pg = json.load(urllib.request.urlopen(req, timeout=30))["query"]["pages"][0]
    ii = (pg.get("imageinfo") or [{}])[0]
    md = ii.get("extmetadata", {})
    g = lambda k: re.sub("<[^>]+>", "", md.get(k, {}).get("value", "")).strip()[:70]
    print(f"{f[:58]:58} | {g('LicenseShortName'):14} | {g('Artist')} | attrib_required={g('AttributionRequired')} | {'FOUND' if ii else 'MISSING'}")
    time.sleep(2)
