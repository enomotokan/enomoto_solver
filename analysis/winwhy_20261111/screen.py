"""WINWHY: 試作 ENOMOTO_MIP_CLQEXT_KNAP の当たる問題を探す。logs/<問題>.<タグ>.log.gz から、足したクリーク行の数・拡張の数・根の LP・
木に入る直前の下界・終わりの状態を取り出して並べる。使い方: python screen.py タグ1 [タグ2]  (タグ2 があれば並べて比べる)"""
import gzip, json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
def get(p, tag):
    f = D / "logs" / f"{p}.{tag}.log.gz"
    if not f.exists(): return None
    r = {}
    try: L = gzip.open(f, "rt", errors="replace").read().splitlines()
    except Exception: return None  # noqa: BLE001
    for l in L:
        m = re.search(r"CLQEXT_KNAP added (\d+)", l)
        if m: r["knap"] = int(m.group(1))
        m = re.search(r"clique extension: (\d+) rows extended, (\d+) nonzeros added.*?(\d+) duplicate and (\d+) dominated", l)
        if m and "ext" not in r: r["ext"] = [int(x) for x in m.groups()]
        m = re.search(r"MIP: root LP (\S+) \(", l)
        if m and "root" not in r:
            try: r["root"] = float(m.group(1))
            except ValueError: pass
        m = re.search(r"cut loop done: .*obj \S+ -> (\S+)", l)
        if m: r["cut"] = float(m.group(1))
        if l[9:].startswith("{"):
            j = json.loads(l[9:]); r["st"] = j.get("status"); r["t"] = j.get("time"); r["rb"] = j.get("root_bound")
    return r
tags = sys.argv[1:]
ps = [p for p, _ in json.loads((D / "shards.json").read_text())["sets"]["scr"]]
for p in ps:
    rs = [get(p, t) for t in tags]
    if rs[0] is None: continue
    def s(r):
        if not r: return "-"
        return f"knap {r.get('knap', '-'):>5} ext {r.get('ext')} root {r.get('root', float('nan')):.6g} cut {r.get('cut', float('nan')):.6g} {r.get('st')} {r.get('t') and round(r['t'], 1)}"
    print(f"{p:28}", " | ".join(s(r) for r in rs))
