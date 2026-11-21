"""RR_TREE=1 の run_root.py の結果の def と dc の比べ: python cmp_tree.py 出力名
両方解けた問題の時間の比 (幾何平均、1 秒以上)、片方だけ解けた問題、解けない問題の下界と解、誤答の疑い (最適値の食い違い) を出す。"""
import json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {}
for l in open(D / f"{sys.argv[1]}.jsonl"):
    r = json.loads(l); rows.setdefault(r["problem"], {})[r["cfg"]] = r
both, only, rt, bad, lines = 0, [], [], [], []
for p in sorted(rows):
    a, b = rows[p].get("def"), rows[p].get("dc")
    if not a or not b: continue
    oa, ob = a["status"] == "optimal", b["status"] == "optimal"
    if oa and ob:
        both += 1
        if abs(a["objective"] - b["objective"]) > 1e-6 * max(1, abs(a["objective"])): bad.append(p)
        if max(a["time"], b["time"]) >= 1: rt.append(math.log(b["time"] / a["time"]))
    elif oa != ob: only.append((p, "def" if oa else "dc"))
    lines.append(f"{p:24s} {a['status']:>10s} {b['status']:>10s} {a['time'] or 0:8.2f} {b['time'] or 0:8.2f} {a['nodes'] or 0:>8} {b['nodes'] or 0:>8} {str(a['bound']):>20s} {str(b['bound']):>20s} {str(a['objective']):>20s} {str(b['objective']):>20s} {b['dse_cost']}")
print(f"{'問題':24s} {'def':>10s} {'dc':>10s} {'秒 def':>8s} {'秒 dc':>8s} {'節 def':>8s} {'節 dc':>8s} {'下界 def':>20s} {'下界 dc':>20s} {'解 def':>20s} {'解 dc':>20s} skip/built/noest")
print("\n".join(lines))
g = math.exp(sum(rt) / len(rt)) if rt else float("nan")
print(f"両方解けた {both} 問、時間の比 (1 秒以上の {len(rt)} 問) の幾何平均 {g:.3f}; 片方だけ解けた {only}; 最適値の食い違い {bad}")
