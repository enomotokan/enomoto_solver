"""OBJINT の分担表を作る。同じ (問題, 種) の回は同じ分担で続けて回す (機械のずれを対で消す)。
P (58 問 × 種 1〜3 × {def, oi}、60 秒): python plan.py P N → shards_P.json
T (刻みが新しく付く問題 × 種 1・2 × {def, oi, oig, highs}、600 秒、ログつき): python plan.py T N 問題 [問題 ...] → shards_T.json"""
import glob, json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
EST = {}
def est(p, c):
    e = EST.get(p, {})
    return e.get(c, e.get("*", 625))
def balance(tasks, n, out):
    groups = defaultdict(list)
    for t in tasks: groups[(t[0], t[2])].append(t)
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0], t[1]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) for t in ts]; load[i] += sum(est(t[0], t[1]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(tasks), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
if sys.argv[1].startswith("P"):
    sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
    from probs import PROBS  # noqa: E402
    T2 = defaultdict(list)
    for f in glob.glob(str(WT / "analysis/cand1111_20261111/res_P_s*.jsonl")):
        for l in open(f):
            r = json.loads(l)
            if r.get("cfg") == "def" and isinstance(r["result"].get("time"), (int, float)): T2[r["problem"]].append(min(r["result"]["time"], 60))
    EST.update({p: {"*": (sum(T2[p]) / len(T2[p]) if T2[p] else 40) + 6} for p in PROBS})
    cand = sys.argv[3] if len(sys.argv) > 3 else "oi"
    out = "shards_P.json" if cand == "oi" else f"shards_P{cand}.json"
    balance([(p, c, s, 60) for p in PROBS for s in (1, 2, 3) for c in ["def", cand]], int(sys.argv[2]), out)
CT = ["def", "oi"]  # T の設定 (HiGHS は hiloss・mitt600 の回を使う)
if sys.argv[1] == "T":
    balance([(p, c, s, 600) for p in sys.argv[3:] for s in (1, 2) for c in CT], int(sys.argv[2]), "shards_T.json")
