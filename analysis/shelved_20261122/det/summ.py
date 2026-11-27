"""det.jsonl とログから、def と ov・ipmxm の決定的な数え (道が同じか) を比べる。python summ.py > det.txt
指紋: 根の LP (値・反復)、カットのラウンドの LP 反復と目的値、外側の根の止まり方と手間、終わりのノード数・下界・目的値、
最後の進みの行の数え (lp_iters・sb_iters・node_iters・heur_iters・dive_iters。時刻・メモリを除く)"""
import gzip, json, re
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
for l in open(D / "det.jsonl"):
    r = json.loads(l); R[r["problem"]][r["cfg"]] = r
def last_counts(p, c):
    t = gzip.open(D / "logs" / f"{p}_{c}.log.gz", "rt").read()
    m = re.findall(r"MIP:\s+nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+) lp_iters (\d+) sb_iters (\d+) .*?node_iters (\d+) .*?heur_iters (\d+) dive_iters (\d+)", t)
    xo = len(re.findall(r"(?i)crossover", t)); ipm = len(re.findall(r"(?i)interior|\bIPM\b", t))
    return (m[-1] if m else None), xo, ipm
def key(r, lc):
    return (r["rootlp"][:1], r["rounds"], r["cutstop"], r["status"], r["nodes"], r["bound"], r["objective"], lc)
print("| 問題 | 既定 (状態・ノード・根の下界・下界・暫定解) | ov は同じ道か | ipmxm は同じ道か | ipmxm で違った最初のところ | ログの crossover/内点法の行 (def→ipmxm) |")
print("|---|---|---|---|---|---|")
for p, d in R.items():
    if not all(c in d for c in ("def", "ov", "ipmxm")): continue
    lc = {c: last_counts(p, c) for c in d}
    k = {c: key(d[c], lc[c][0]) for c in d}
    a = d["def"]
    def first_diff(x, y):
        names = ["根の LP", "カットのラウンド", "根の止まり方・手間", "状態", "ノード数", "下界", "暫定解", "進みの数え"]
        for n, u, v in zip(names, x, y):
            if u != v: return n
        return "-"
    print(f"| {p} | {a['status']}・{a['nodes']}・{a['root_bound']:.6g}・{a['bound']}・{a['objective']} | {'同じ' if k['ov'] == k['def'] else '違う (' + first_diff(k['def'], k['ov']) + ')'} | "
          f"{'同じ' if k['ipmxm'] == k['def'] else '違う'} | {first_diff(k['def'], k['ipmxm'])} | {lc['def'][1]}/{lc['def'][2]} → {lc['ipmxm'][1]}/{lc['ipmxm'][2]} |")
