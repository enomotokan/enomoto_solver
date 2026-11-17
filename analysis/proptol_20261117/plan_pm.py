"""rtc の P/M の分担表: python plan_pm.py NP NM → shards_PX.json (58 問 × 種 1〜3 × {def, rtc}、60 秒) と shards_MX.json
(analysis/pcd_20261117 の M と同じ 26 問 × 種 1・2 × {def, rtc}、600 秒)。同じ (問題, 種) の 2 設定は同じ分担で続けて解き、順は種で入れ替える。
見積りは cand1119 の回の def の時間 (時間切れは制限時間、記録のない問題は制限時間) × 2 + 立ち上げ。実行は python run.py PX K / MX K。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "pcd_20261117"))
# pcd の plan.py の問題の一覧と時間の読み込みだけを使う (分担表を書く部分は読み込まない)
src = (D.parent / "pcd_20261117/plan.py").read_text().split("NP, NM = int(sys.argv[1])")[0]
g = {"__file__": str(D.parent / "pcd_20261117/plan.py")}
exec(compile(src, "pcdplan", "exec"), g)
PROBS, MPROBS, times = g["PROBS"], g["MPROBS"], g["times"]
CF = ["def", "rtc"]
def balance(groups, est, n, out):
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for key, runs in sorted(groups, key=lambda kv: -est(kv[0][0]) * len(kv[1])):
        i = min(range(n), key=lambda j: load[j]); shards[i] += runs; load[i] += est(key[0]) * len(runs)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, n, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
def order(p, s, tl):
    return [[p, c, s, tl] for c in (CF if s % 2 else CF[::-1])]
NP, NM = int(sys.argv[1]), int(sys.argv[2])
TP = times(D.parent / "cand1119_20261117/res_P_s*.jsonl", "def", 60)
balance([((p, s), order(p, s, 60)) for p in PROBS for s in (1, 2, 3)], lambda p: TP.get(p, 60.0) + 8, NP, "shards_PX.json")
TM = times(D.parent / "cand1119_20261117/res_M_s*.jsonl", "def", 600)
balance([((p, s), order(p, s, 600)) for p in MPROBS for s in (1, 2)], lambda p: TM.get(p, 600.0) + 40, NM, "shards_MX.json")
