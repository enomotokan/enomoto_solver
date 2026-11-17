"""行の種類ごとの数え上げの集計: python summ.py > summ.txt。
(1) 木に入るとき (根のカットのループの後の LP 解): 種類ごとに、行・非零 (全問の和と、問題ごとの割合)、不等式の余裕の分布 (0 / ≤1e-3 / ≤1e-1 / >1e-1)、スラックが基底の割合。
(2) 大域の境界の下で冗長な行: 木に入るとき・60 秒・終わりの数 (固定されていない列が 0 本の行を除いた非零も)。
(3) 問題ごとの表: 余裕 > 1e-1 の不等式の非零の割合 (種類別) と、終わりの冗長な行の非零の割合。"""
import json, re, collections
from pathlib import Path
D = Path(__file__).resolve().parent
K = ["clique", "vb", "knap", "int", "mixed", "cont", "eq", "range"]
rows = [json.loads(l) for l in open(D / "res.jsonl") if l.strip()]
def parse(line):
    m = re.search(r"ROWTYPES (\S+(?: start)?): t ([0-9.]+)s nodes (\d+) rows (\d+) nnz (\d+) fixed int (\d+)/(\d+); (.*) \(", line)
    if not m: return None
    d = {"label": m.group(1), "t": float(m.group(2)), "nodes": int(m.group(3)), "rows": int(m.group(4)), "nnz": int(m.group(5)), "fixed": int(m.group(6)), "nint": int(m.group(7)), "k": {}}
    for part in m.group(8).split("; "):
        mm = re.match(r"(\w+) (\d+) nnz (\d+) red (\d+) \(nnz (\d+)\)(?: basic (\d+) slack0/≤1e-3/≤1e-1/>1e-1 (\d+)/(\d+)/(\d+)/(\d+))?", part)
        if mm:
            g = [int(x) if x is not None else 0 for x in mm.groups()[1:]]
            d["k"][mm.group(1)] = dict(n=g[0], nnz=g[1], red=g[2], rednnz=g[3], basic=g[4], s=g[5:9])
    return d
P = {}
for r in rows:
    snaps = {d["label"]: d for d in (parse(l) for l in r["lines"]) if d}
    P[r["problem"]] = (r, snaps)
print(f"{len(P)} 問、木に入った問題 {sum(1 for _, s in P.values() if 'tree start' in s)}\n")
tot = collections.defaultdict(lambda: [0] * 9)
for p, (r, s) in P.items():
    d = s.get("tree start")
    if not d: continue
    for k, v in d["k"].items():
        t = tot[k]; t[0] += v["n"]; t[1] += v["nnz"]; t[2] += v["basic"]
        for i in range(4): t[3 + i] += v["s"][i]
        t[7] += sum(v["s"]); t[8] += v["red"]
print("## (1) 木に入るとき (全問の和)")
print(f"{'種類':8s} {'行':>9s} {'非零':>10s} | {'LP を見た行':>10s} {'基底':>6s} | 余裕 0 / ≤1e-3 / ≤1e-1 / >1e-1 (割合) | 冗長")
for k in K:
    if k not in tot: continue
    t = tot[k]; n = max(t[7], 1)
    print(f"{k:8s} {t[0]:9d} {t[1]:10d} | {t[7]:10d} {100*t[2]/n:5.1f}% | {100*t[3]/n:5.1f} / {100*t[4]/n:5.1f} / {100*t[5]/n:5.1f} / {100*t[6]/n:5.1f} | {t[8]}")
print("\n## (2)(3) 問題ごと: 余裕 > 1e-1 の行 (種類: 本数) と、冗長な行 (木に入るとき → 60 秒 → 終わり、括弧は固定されていない列のある行の非零の割合)")
for p, (r, s) in sorted(P.items(), key=lambda kv: -(kv[1][1].get("tree start", {"nnz": 0})["nnz"])):
    d = s.get("tree start")
    if not d: print(f"{p:22s} 木に入らない ({r['result'].get('status')})"); continue
    big = ", ".join(f"{k} {v['s'][3]}" for k, v in d["k"].items() if v["s"][3])
    red = []
    for lab in ("tree start", "mid", "end"):
        e = s.get(lab)
        if not e: red.append("-"); continue
        rn = sum(v["red"] for v in e["k"].values()); rz = sum(v["rednnz"] for v in e["k"].values())
        red.append(f"{rn} ({100*rz/max(e['nnz'],1):.1f}%, fix {e['fixed']}/{e['nint']}, t {e['t']:.0f})")
    print(f"{p:22s} 行 {d['rows']:6d} 非零 {d['nnz']:7d} | 余裕>1e-1: {big or '-'} | 冗長: {' → '.join(red)}")
