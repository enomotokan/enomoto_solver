"""detect_<集合>.jsonl を集計: 設定ごとに生成元・軌道が def と違う問題、同点の行・割った行のある問題の数。"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
which = sys.argv[1]
R = defaultdict(dict)
for l in open(D / f"detect_{which}.jsonl"):
    r = json.loads(l); R[r["problem"]][r["cfg"]] = r
def g(r, k):
    return r.get(k, "-") if r else "-"
print(f"問題 {len(R)}、検出まで届かなかった回: " + ", ".join(f"{p}/{c} ({r['rc']})" for p, cs in R.items() for c, r in cs.items() if not r.get("stopped")))
nt = sum(1 for cs in R.values() if (cs.get("sign") or {}).get("ties", 0) > 0)
ns = sum(1 for cs in R.values() if (cs.get("scale") or {}).get("scaled", 0) > 0)
print(f"同点の行がある問題 {nt}、倍率で割った行がある問題 {ns}")
print("| 問題 | 行 | 同点の行 | 割った行 | 生成元 def/zero/sign/scale/both | 軌道 def/zero/sign/scale/both | 検出の秒 def/zero/sign/scale/both |")
print("|---|---|---|---|---|---|---|")
for p, cs in sorted(R.items()):
    d = cs.get("def") or {}
    diff = any((cs.get(c) or {}).get(k) != d.get(k) for c in ("zero", "sign", "scale", "both") for k in ("gens", "orbits"))
    if not diff and "--all" not in sys.argv: continue
    print(f"| {p} | {g(cs.get('both'),'rows')} | {g(cs.get('sign'),'ties')} | {g(cs.get('scale'),'scaled')} | "
          + "/".join(str(g(cs.get(c), 'gens')) for c in ("def", "zero", "sign", "scale", "both")) + " | "
          + "/".join(str(g(cs.get(c), 'orbits')) for c in ("def", "zero", "sign", "scale", "both")) + " | "
          + "/".join(str(g(cs.get(c), 't_detect')) for c in ("def", "zero", "sign", "scale", "both")) + " |")
tot = {c: sum((cs.get(c) or {}).get("t_detect", 0) for cs in R.values()) for c in ("def", "zero", "sign", "scale", "both")}
print("検出の秒の合計: " + ", ".join(f"{c} {v:.2f}" for c, v in tot.items()))
