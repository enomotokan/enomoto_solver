"""check55.jsonl を既定 / notiny で突き合わせる: 固定が働いた問題、前処理後の大きさが変わった問題、目的値と状態が変わった問題。"""
import json, sys
from collections import defaultdict
R = defaultdict(dict)
for l in open(sys.argv[1] if len(sys.argv) > 1 else "check55.jsonl"):
    d = json.loads(l); R[d["problem"]][d["config"]] = d
print("problems", len(R))
bad = [(p, c, d["bad"]) for p, v in R.items() for c, d in v.items() if d.get("bad")]
print("bad (bound>opt / wrong optimal / infeasible x):", bad)
crash = [(p, c, d["status"]) for p, v in R.items() for c, d in v.items() if d["status"] not in ("optimal", "time_limit", "infeasible")]
print("crash/other:", crash)
print("\n## 固定が働いた問題 (既定)")
for p, v in sorted(R.items()):
    d = v.get("default")
    if d and d["tiny"]:
        print(p, d["tiny"])
print("\n## 前処理後の大きさ (最初の 'rows a -> b' 行) が変わった問題")
for p, v in sorted(R.items()):
    a, b = v.get("default"), v.get("notiny")
    if a and b and a["presolve"][:1] != b["presolve"][:1]:
        print(p, "default:", a["presolve"][:1], "| notiny:", b["presolve"][:1])
print("\n## 状態・目的値が変わった問題 (相対 1e-6 を越える差)")
def f(x): return x if isinstance(x, (int, float)) else None
for p, v in sorted(R.items()):
    a, b = v.get("default"), v.get("notiny")
    if not (a and b): continue
    oa, ob = f(a.get("objective")), f(b.get("objective"))
    diff = (oa is None) != (ob is None) or (oa is not None and abs(oa - ob) > 1e-6 * max(1, abs(ob)))
    if a["status"] != b["status"] or diff:
        print(p, "default:", a["status"], oa, a.get("bound"), "| notiny:", b["status"], ob, b.get("bound"), "| ref", a.get("ref"))
