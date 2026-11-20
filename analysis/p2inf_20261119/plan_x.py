"""P2-INF 8 節の続き: M で cwrh0 が全種で遅い・速い問題に種を足す分担表 (python plan_x.py N → shards_MCWRH0X.json)。
{def, cwrh0} × 種 3〜8、600 秒・4 スレッド。遅れ (速さ) が運かを確かめる。見積りは MCWRH0 の 2 者の平均時間 + 30 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1])
PROBS = ["neos-933966", "neos5", "snp-02-004-104", "gen-ip054", "rocI-4-11", "mcsched", "neos-662469"]  # 遅: 前 3 問 (snp は 2 種とも 1.25 倍以上)、速: 後 4 問
T = defaultdict(list)
for f in glob.glob(str(D / "res_MCWRH0_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); T[r["problem"]].append(min(r["result"].get("time") or 600, 600))
est = lambda p: sum(T[p]) / len(T[p]) + 30
groups = {(p, s): [(p, c, s) for c in (["def", "cwrh0"] if s % 2 else ["cwrh0", "def"])] for p in PROBS for s in range(3, 9)}
load = [0.0] * N; shards = [[] for _ in range(N)]
for (p, s), ts in sorted(groups.items(), key=lambda kv: -est(kv[0][0])):
    i = min(range(N), key=lambda j: load[j]); shards[i] += [list(t) + [600] for t in ts]; load[i] += 2 * est(p)
(D / "shards_MCWRH0X.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(groups), "組", sum(map(len, shards)), "回", [round(x / 3600, 2) for x in load])
