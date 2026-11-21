"""res_tg.jsonl の一覧: python show.py [TAG の部分文字列]。根のギャップ = |opt - root| / max(|opt|, 1)。"""
import json, sys
f = sys.argv[1] if len(sys.argv) > 1 else ""
for l in open(__import__("pathlib").Path(__file__).parent / "res_tg.jsonl"):
    r = json.loads(l); x = r["result"]
    if f not in r["tag"]: continue
    o = r["opt"]; rb = x.get("root_bound")
    g = abs(o - rb) / max(abs(o), 1) if (o is not None and rb is not None) else None
    print(f"{r['problem']:18s} s{r['seed']} {r['solver']:5s} {r['tag']:14s} {x['status'][:8]:8s} {x.get('nodes'):>8} {x.get('time') or 0:7.1f}s root {rb if rb is None else round(rb, 4)} gap {g if g is None else round(g, 4)} obj {x.get('objective')}")
