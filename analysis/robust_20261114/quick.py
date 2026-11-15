"""試作の組と基準の回ごとの時間を問題ごとに並べる: python quick.py スレッド 組 [基準の組=res_t<スレッド>.jsonl]
回の並びは p0s1 p0s2 p0s3 p1s1 p2s1 p3s1。T は時間切れ。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
th, name = sys.argv[1], sys.argv[2]
basef = D / (f"res_t{th}_{sys.argv[3]}.jsonl" if len(sys.argv) > 3 else f"res_t{th}.jsonl")
def load(f):
    r = {}
    if f.exists():
        for l in f.read_text().splitlines():
            if l.strip():
                x = json.loads(l); r[(x["problem"], x["perm"], x["seed"])] = x["result"]
    return r
V = [(0, 1), (0, 2), (0, 3), (1, 1), (2, 1), (3, 1)]
a, b = load(basef), load(D / f"res_t{th}_{name}.jsonl")
def f(res):
    if res is None: return "-"
    t = min(res.get("time") or 60, 60)
    return f"{t:.1f}" if res.get("status") == "optimal" else "T"
for p in sorted({k[0] for k in b}):
    print(f"{p:22s} base {' '.join(f(a.get((p, *v))) for v in V):40s} {name} {' '.join(f(b.get((p, *v))) for v in V)}")
