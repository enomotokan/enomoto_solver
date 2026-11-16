"""M で cand1117 が全種で遅かった問題に種 3・4 を足す追加の分担 (cand1116 の shards_X と同じ形): python planX.py [問題 ...] → shards_X.json
問題を与えなければ、今ある res_M_s*.jsonl から「cand1117 / cand1116 の比 (shift 1、時間切れは 600 秒) が全種で > 1.25」の問題を選ぶ。
1 分担 = 1 (問題, 種) × {cand1116, cand1117, highs} (600 秒、最大 3 × 600 秒 + 立ち上げ ≈ 32 分)。回し方 python run.py X K、集計 python summ.py MX (M と X を合わせる)。"""
import json, glob, sys, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
def st(r): return str(r["result"].get("status") or "").lower()
def t(r): return min(r["result"]["time"], r["tl"]) if st(r) in ("optimal", "infeasible") else r["tl"]
probs = sys.argv[1:]
if not probs:
    R = {(r["problem"], r["cfg"], r["seed"]): r for f in glob.glob(str(D / "res_M_s*.jsonl")) for l in open(f) if l.strip() for r in [json.loads(l)]}
    lr = defaultdict(list)
    for (p, c, s), r in R.items():
        if c == "cand1117" and (p, "cand1116", s) in R:
            lr[p].append(math.log((t(r) + 1) / (t(R[(p, "cand1116", s)]) + 1)))
    probs = sorted(p for p, v in lr.items() if len(v) >= 2 and all(x > math.log(1.25) for x in v))
shards = []
for p in probs:
    for s in (3, 4):
        c = ["cand1116", "cand1117", "highs"]; k = s % 3
        shards.append([[p, x, s, 600] for x in c[k:] + c[:k]])
(D / "shards_X.json").write_text(json.dumps({"shards": shards, "probs": probs}, indent=0))
print("shards_X.json:", len(probs), "問", probs, "×", "種 3・4 =", len(shards), "分担 (K = 0〜%d)" % (len(shards) - 1))
