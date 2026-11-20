"""M (Mittelmann MIP 48 問、analysis/cand1116_20261115 の M と同じ組) の分担表: python planM.py N [種...] → shards_M.json
設定 plain・nbg (run1.py の CFGS)、600 秒・4 スレッド。同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。
見積りは cand1116 の M の回 (cand1116) の時間 + 30 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1]); SEEDS = [int(x) for x in sys.argv[2:]] or [1]
T = defaultdict(list)
for f in glob.glob(str(D.parent / "cand1116_20261115" / "res_M_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1116":
            T[r["problem"]].append(min(r["result"].get("time") or 600, 600))
est = {p: sum(v) / len(v) + 30 for p, v in T.items()}
groups = {(p, s): ([(p, "plain", s), (p, "nbg", s)] if s % 2 else [(p, "nbg", s), (p, "plain", s)]) for p in est for s in SEEDS}
load = [0.0] * N; shards = [[] for _ in range(N)]
for (p, s), ts in sorted(groups.items(), key=lambda kv: -est[kv[0][0]]):
    i = min(range(N), key=lambda j: load[j]); shards[i] += [list(t) + [600] for t in ts]; load[i] += 2 * est[p]
(D / "shards_M.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(est), "問", sum(len(s) for s in shards), "回、見積り (時間):", [round(x / 3600, 2) for x in load])
