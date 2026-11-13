#!/usr/bin/env python3
"""List Wikimedia Commons images for a species, keeping only CC0/PD/CC-BY licences."""
import json, sys, urllib.parse, urllib.request, re

UA = "reef-aquarium-asset-search/0.1 (personal hobby project)"
API = "https://commons.wikimedia.org/w/api.php"
OK = re.compile(r"^(cc0|public domain|pd|cc[- ]by[- ]\d(\.\d)?$|cc by \d(\.\d)?$)", re.I)

def api(**p):
    p.update(format="json", formatversion="2")
    req = urllib.request.Request(API + "?" + urllib.parse.urlencode(p), headers={"User-Agent": UA})
    return json.load(urllib.request.urlopen(req, timeout=30))

def search(term, limit=40):
    r = api(action="query", generator="search", gsrsearch=f"{term} filetype:bitmap", gsrnamespace=6,
            gsrlimit=limit, prop="imageinfo", iiprop="url|size|extmetadata", iiurlwidth=1200)
    out = []
    for pg in r.get("query", {}).get("pages", []):
        ii = (pg.get("imageinfo") or [{}])[0]
        md = ii.get("extmetadata", {})
        lic = md.get("LicenseShortName", {}).get("value", "")
        if not OK.search(lic.strip()) or "SA" in lic.upper().split():
            continue
        if "-sa" in lic.lower() or "sa " in lic.lower():
            continue
        out.append(dict(title=pg["title"], lic=lic, w=ii.get("width"), h=ii.get("height"),
                        artist=re.sub("<[^>]+>", "", md.get("Artist", {}).get("value", ""))[:60],
                        url=ii.get("url"), page=ii.get("descriptionurl")))
    return out

if __name__ == "__main__":
    for term in sys.argv[1:]:
        res = [r for r in search(term) if (r["w"] or 0) >= 1200]
        print(f"\n## {term}: {len(res)} usable")
        for r in res[:8]:
            print(f"  {r['w']}x{r['h']} [{r['lic']}] {r['title']} -- {r['artist']}")
