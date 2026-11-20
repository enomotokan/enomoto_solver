"""確かめの計測 (README 12 節) の分担表: 連続列の割合 < 0.6 を足した判定で、主の根が HiGHS の選び方になる問題だけ (feat.jsonl の札 dec2)。
HM: M の 10 問 × 種 5・6 × {def, tsig2}、600 秒。HP: P の 13 問 × 種 7・8 × {def, tsig2}、60 秒。今までに使っていない種。
1 分担 ≤ 1.5 時間 (見積りは res_M・res_P の def と tsig の平均 + 30 秒・8 秒)。python plan_h.py → shards_HM.json・shards_HP.json"""
import json, glob
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
HM = ["csched008", "fiball", "mcsched", "mzzv11", "mzzv42z", "neos-873061", "neos-933966", "ran14x18-disj-8", "rococoC10-001000", "satellites2-60-fs"]
HP = ["assign1-5-8", "blp-ar98", "bppc4-08", "cvs16r128-89", "graph20-20-1rand", "mc11", "mzzv42z", "neos-3083819-nubu", "neos-911970",
      "physiciansched6-2", "qnet1", "ran14x18-disj-8", "rout"]
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(D / pat)):
        for l in open(f):
            r = json.loads(l); T[r["problem"]].append(min(r["result"].get("time") or cap, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def plan(probs, seeds, tl, pat, add, cap_h, out):
    T = times(pat, tl)
    groups = [((p, s), [(p, c, s, tl) for c in (["def", "tsig2"] if s % 2 else ["tsig2", "def"])]) for p in probs for s in seeds]
    est = lambda p: T.get(p, tl) + add
    groups.sort(key=lambda g: -2 * est(g[0][0]))
    n = 1
    while True:
        load = [0.0] * n; sh = [[] for _ in range(n)]
        for (p, s), ts in groups:
            i = min(range(n), key=lambda j: load[j]); sh[i] += [list(t) for t in ts]; load[i] += 2 * est(p)
        if max(load) <= cap_h * 3600 or n >= 20: break
        n += 1
    (D / out).write_text(json.dumps({"shards": sh, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, n, "分担", sum(len(x) for x in sh), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
plan(HM, (5, 6), 600, "res_M_s*.jsonl", 30, 1.5, "shards_HM.json")
plan(HP, (7, 8), 60, "res_P_s*.jsonl", 8, 1.5, "shards_HP.json")
