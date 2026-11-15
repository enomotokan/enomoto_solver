"""M の組 (analysis/cand1114_20261113/plan.py の M、48 問) × 種 1・2 × {cand1114, res} 600 秒の分担表を作る: python planM.py N → shards_M.json。
同じ問題の回は同じ分担に入れる (時期・機械のずれを対で消す)。時間の見積りは cand1114 の res_M (cand1114 の行)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
C = D.parent / "cand1114_20261113"
N = int(sys.argv[1])
T = defaultdict(list)
for f in glob.glob(str(C / "res_M_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1114": T[r["problem"]].append(min(r["result"].get("time") or 600, 600))
M = sorted(T)
est = {p: 2 * (sum(T[p]) / len(T[p]) + 30) * 2 for p in M}
gl = sorted(M, key=lambda p: -est[p]); load = [0.0] * N; sh = [[] for _ in range(N)]
for p in gl:
    i = min(range(N), key=lambda j: load[j]); load[i] += est[p]
    sh[i] += [[p, c, s, 600] for s in (1, 2) for c in ("cand1114", "res")]
(D / "shards_M.json").write_text(json.dumps({"shards": sh, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(M), "問", sum(map(len, sh)), "回", [round(x / 3600, 2) for x in load])
