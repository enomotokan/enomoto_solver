"""t1gap の案 1・3 の試作の広い計測の分担表 (rootgap_20261121/plan.py の形)。
python plan.py P 6 設定,設定...  → shards_P.json (MIPLIB の 58 問 × 種 1〜3、60 秒)
python plan.py M 20 設定,設定... → shards_M.json (Mittelmann 186 問 × 種 1、600 秒)
見積りは analysis/vsscip_20261120 の def (4 スレッド) と def_t1 (1 スレッド) の時間 (解けなかった回は上限) + 8 秒 (P)・30 秒 (M)。
設定の名前が _t1 で終われば def_t1 の時間、ほかは def の時間で見積もる。設定の順は問題・種で回して入れ替える。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parents[1]
sys.path.insert(0, str(A / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
SET, NS, CF = sys.argv[1], int(sys.argv[2]), sys.argv[3].split(",")
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap):
    T = {}
    for f in glob.glob(str(pat)):
        for l in open(f):
            if not l.strip(): continue
            r = json.loads(l)
            if r["cfg"] in ("def", "def_t1") and "time" in r["result"]:
                T[(r["cfg"], r["problem"], r["seed"])] = min(r["result"]["time"], cap) if solved(r) else cap
    return T
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(*t) for t in ts)
    (D / out).write_text(json.dumps({"cfgs": CF, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜",
          max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
def rot(i): k = i % len(CF); return CF[k:] + CF[:k]
def base(c): return "def_t1" if c.endswith("_t1") else "def"
if SET == "P":
    T = times(A / "vsscip_20261120" / "res_P_s*.jsonl", 60)
    balance({(p, s): [(p, c, s) for c in rot(i + s)] for i, p in enumerate(PROBS) for s in (1, 2, 3)},
            lambda p, c, s: T.get((base(c), p, s), T.get((base(c), p, 1), 60)) + 8, NS, "shards_P.json", 60)
elif SET == "M":
    T = times(A / "vsscip_20261120" / "res_M_s*.jsonl", 600)
    MP = sorted({x for part in json.loads((A / "cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
    balance({(p, 1): [(p, c, 1) for c in rot(i)] for i, p in enumerate(MP)},
            lambda p, c, s: T.get((base(c), p, s), 600) + 30, NS, "shards_M.json", 600)
