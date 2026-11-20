"""SCIP-GATE の分担表 (scipslow_20261119/plan.py の写し): python plan.py NM NP [設定,..] [接尾辞] → shards_M<接尾辞>.json・shards_P<接尾辞>.json (NM = 0 なら P だけ)
P: MIPLIB の 58 問 × 種 1〜3 × 設定、60 秒 (4 スレッド)。
M: e が働きうる問題 × 種 1・2 × 設定、600 秒。問題は下の A ∪ B ∪ C (workscan・bigmscan の数え上げ)。
設定は 3 番目の引数 (既定 def,vbce。1 回目の計測は def,vb,vbc,vbcl で P だけ)。
同じ (問題, 種) の 4 組は同じ分担で続けて解き、順は種で回す。見積り: vshighs_20261117 の def の時間 (+ 30 秒・8 秒)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
VS = D.parents[0] / "vshighs_20261117"
sys.path.insert(0, str(D.parents[0] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
CF = sys.argv[3].split(",") if len(sys.argv) > 3 else ["def", "vbce"]  # 1 回目の計測は def,vb,vbc,vbcl
SUF = sys.argv[4] if len(sys.argv) > 4 else "2"  # 出力の接尾辞 (2 回目は shards_P2.json・shards_M2.json → run.py P2 K、res_P2_sK.jsonl。1 回目は "")
def times(pat, cfg, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != cfg: continue
            t = r["result"].get("time") or cap
            T[r["problem"]].append(min(t, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0], t[1]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0], t[1]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
def order(p, s, c):
    k = s % len(c)
    return [(p, x, s) for x in c[k:] + c[:k]]
WS = [json.loads(l) for f in sorted(glob.glob(str(D / "workscan.jsonl_*"))) for l in open(f) if l.strip()]
BM = [json.loads(l) for f in sorted(glob.glob(str(D / "bigmscan.jsonl_*"))) for l in open(f) if l.strip()]
# e が働きうる M の問題 (2 回目の計測): (a) VB01 が根の前に解を出した、(b) pseudoobj が固定した (workscan、VB01=b + VBCLQ=1)、
# (c) 大 M の行がある (bigmscan) の和。P に入っていないもの
A = {r["problem"] for r in WS if r["set"] == "M" and "found" in (r.get("vb01") or [])}
B = {r["problem"] for r in WS if r["set"] == "M" and any(x["fixed"] > 0 or x["proved"] for x in (r.get("vbclq") or []))}
C = {r["problem"] for r in BM if r["set"] == "M" and r.get("bigm_rows", 0) > 0}
MPROBS = sorted(A | B | C)
print("M の問題", len(MPROBS), "(VB01 の解", len(A), "・pseudoobj", len(B), "・大 M", len(C), ")")
TM = times(VS / "res_M_s*.jsonl", "def", 600)
TA = times(D.parents[0] / "vsscip_20261117" / "res_M_s*.jsonl", "def", 600)
def estM(p, c): return TM.get(p, TA.get(p, 600.0)) + 30
if NM > 0:  # NM = 0 なら M の分担表は作らない (P だけ先に出すとき)
    balance({(p, s): order(p, s, CF) for p in MPROBS for s in (1, 2)}, estM, NM, f"shards_M{SUF}.json", 600)
TP = times(VS / "res_P_s*.jsonl", "def", 60)
def estP(p, c): return TP.get(p, 48.0) + 8
balance({(p, s): order(p, s, CF) for p in PROBS for s in (1, 2, 3)}, estP, NP, f"shards_P{SUF}.json", 60)
