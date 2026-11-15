"""(a) の表: 問題・種ごとに既定と FPR の最初の解の時刻 (外側の時計)・その値・120 秒での値・下界。"""
import json, sys
from collections import defaultdict
R = defaultdict(dict)
for l in open(sys.argv[1] if len(sys.argv) > 1 else "a120.jsonl"):
    r = json.loads(l); R[(r["problem"], r["seed"])][r["cfg"]] = r
print("| 問題 | 種 | 最初の解の時刻 既定 → FPR | 最初の解の値 既定 → FPR | 120 秒の値 既定 → FPR | FPR の別スレッド |")
print("|---|---|---|---|---|---|")
for (p, s), d in sorted(R.items()):
    if "def" not in d or "fpr" not in d: continue
    a, b = d["def"]["result"], d["fpr"]["result"]
    th = [x for x in d["fpr"]["fpr_lines"] if "FPR thread:" in x]
    th = th[0].split("FPR thread: ")[1][:70] if th else ""
    f = lambda v: "-" if v is None else (f"{v:.1f}" if isinstance(v, float) else str(v))
    g = lambda v: "-" if v is None else f"{v:.6g}"
    print(f"| {p} | {s} | {f(a.get('first_solution_time'))} → {f(b.get('first_solution_time'))} | {g(a.get('first_solution_objective'))} → {g(b.get('first_solution_objective'))} "
          f"| {g(a.get('objective'))} → {g(b.get('objective'))} | {th} |")
