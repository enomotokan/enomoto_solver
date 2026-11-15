"""根の下界の 2 回の比べ (root58・rootx・rootsub をまとめる): python summ_sub.py 設定,... → 問題ごとに 設定の 1 回目/2 回目、
行 2 本以上で位数が 2・3・5・7 以外の元のカットの数 (snfm の記録)"""
import gzip, json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
cfgs = sys.argv[1].split(",")
R = defaultdict(dict)
for f in ("root58.jsonl", "rootx.jsonl", "rootsub.jsonl"):
    if (D / f).exists():
        for l in open(D / f):
            r = json.loads(l)
            R[r["problem"]][(r["cfg"], r["rep"])] = r
sub = open(D / "subset.txt").read().strip().split(",")
def rb(r):
    lp = r.get("root_loops") or [] if r else []
    return lp[0]["obj1"] if lp else None
def nm(p, c, rep):
    f = D / "logs" / f"{c}_{p}_60_r{rep}_root.err.gz"
    if not f.exists():
        return "-"
    n = 0; e = 0.0
    for l in gzip.open(f, "rt"):
        m = re.search(r"new_multi (\d+) eff_new_multi (\S+)", l)
        if l.startswith("SEP snfmodk") and m:
            n += int(m.group(1)); e = max(e, float(m.group(2)))
    return f"{n}/{e:.3f}"
print("問題 | " + " | ".join(f"{c} r1 / r2" for c in cfgs) + " | snfm の新しい位数・行 2 本以上のカット数/効き目の最大")
for p in sub:
    cells = []
    for c in cfgs:
        v = [rb(R[p].get((c, k))) for k in (1, 2)]
        cells.append(" / ".join("-" if x is None else f"{x:.6g}" for x in v))
    print(p, "|", " | ".join(cells), "|", nm(p, "snfm", 2))
