"""cand1111 の切り分け (shards_B.json、既定に 1 項目だけ足した only_*) の集計。各 (問題, 種) で def と比べる。shift 10 秒。"""
import json, glob, math, collections
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in glob.glob(str(D / "res_B_s*.jsonl")) for l in open(f)]
def solved(r):
    st = str(r["result"].get("status", "")).lower()
    return st.startswith("optimal") or st.startswith("infeas")
def t(r): return min(r["result"]["time"], 600) if solved(r) else 600
T = collections.defaultdict(dict)
for r in rows: T[(r["problem"], r["seed"])][r["cfg"]] = r
cfgs = json.loads((D / "shards_B.json").read_text())["cfgs"]
print("crash", sum(1 for r in rows if not solved(r) and str(r["result"].get("status")) not in ("time_limit",)), "行数", len(rows))
print("時間 (秒、600 = 時間切れ)")
print(f"{'問題':22s}種 " + " ".join(f"{c.replace('only_', '')[:8]:>8s}" for c in cfgs))
for k in sorted(T):
    print(f"{k[0][:22]:22s}{k[1]}  " + " ".join(f"{t(T[k][c]):8.0f}" if c in T[k] else "       -" for c in cfgs))
print("\ndef に対する比 (shift 10 の幾何平均)、解けた数 (def との差)、1.5 倍以上遅い / 0.67 倍以下に速い (問題,種) の数")
for c in cfgs[1:]:
    ks = [k for k in T if c in T[k] and "def" in T[k]]
    v = [math.log((t(T[k][c]) + 10) / (t(T[k]["def"]) + 10)) for k in ks]
    sv = sum(solved(T[k][c]) for k in ks) - sum(solved(T[k]["def"]) for k in ks)
    slow = sum(x > math.log(1.5) for x in v); fast = sum(x < math.log(1 / 1.5) for x in v)
    print(f"  {c:16s} {math.exp(sum(v) / len(v)):.3f}  解けた {sv:+d}  遅 {slow} 速 {fast}  (n={len(ks)})")
