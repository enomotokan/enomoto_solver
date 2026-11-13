"""cand1114 の確かめの分担表を作る。(P) 58 問 (MIPLIB) × 種 1〜4 × {def, cand1114} 60 秒 + physiciansched6-2 の種 5〜12、
(M) Mittelmann の絞った組 × 種 1・2 × {def, cand1114} 600 秒 + 種 1 の HiGHS。M は同じ問題の回を全部、P は同じ (問題, 種) の def と cand1114 を
同じ分担に入れる (時期・機械のずれを対で消す)。時間の見積りは cand1113 の回 (M: res_M、P: res_P)。使い方: python plan.py NM NP → shards_M.json・shards_P.json
M の組 (理由は README.md):
  - cand1113 で 2 種とも遅かった 5 問 (loo.md・loo2.md で原因の項目を突き止めたもの)
  - unitcal_7 (cand1113 の誤答、直し 40e80904)
  - HiGHS に負けている 12 問 (analysis/highsloss_20261113 と同じ組)
  - cand1113 だけが片方の種で解いた問題・2 種とも速かった問題 (cand1113 の summM.txt の一覧、効きが残るかの確かめ)"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C13 = WT / "analysis" / "cand1113_20261112"
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
SLOW = ["enlight_hard", "neos-1122047", "cbs-cta", "roll3000", "neos-960392"]
WRONG = ["unitcal_7"]
HIGHSLOSS = sorted({p for s in json.loads((WT / "analysis/highsloss_20261113/shards.json").read_text())["shards"] for p, _ in s})
ONESIDE = ["CMS750_4", "csched008", "fast0507", "fiball", "gen-ip054", "hypothyroid-k1", "istanbul-no-cutoff", "mcsched", "neos-1456979",
           "neos-3627168-kasai", "neos-4722843-widden", "neos-662469", "neos-787933", "peg-solitaire-a3", "pg5_34", "ran14x18-disj-8", "rocI-4-11",
           "snp-02-004-104", "supportcase33", "supportcase40", "tr12-30", "traininstance6"]
FAST = ["fastxgemm-n2r6s0t2", "fiball", "h80x6320d", "irp", "istanbul-no-cutoff", "mas74", "mas76", "neos-2978193-inde", "neos-3627168-kasai",
        "neos-4722843-widden", "neos-4738912-atrato", "neos5", "pg5_34", "ran14x18-disj-8", "tbfp-network", "timtab1"]
M = list(dict.fromkeys(SLOW + WRONG + HIGHSLOSS + ONESIDE + FAST))
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est[t[0], t[1]] for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est[t[0], t[1]] for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(C13 / pat)):
        for l in open(f):
            r = json.loads(l); t = r["result"].get("time") or cap
            T[r["problem"], r["cfg"].replace("cand1113", "cand1114")].append(min(t, cap))
    return T
TM = times("res_M_s*.jsonl", 600)
estM = defaultdict(lambda: 630.0, {k: sum(v) / len(v) + 30 for k, v in TM.items()})
missing = [p for p in M if (p, "def") not in TM]
assert not missing, missing
gM = {p: [(p, "def", 1), (p, "cand1114", 1), (p, "highs", 1), (p, "def", 2), (p, "cand1114", 2)] for p in M}
balance(gM, estM, NM, "shards_M.json", 600)
TP = times("res_P_s*.jsonl", 60)
estP = defaultdict(lambda: 48.0, {k: sum(v) / len(v) + 8 for k, v in TP.items()})
gP = {}
for p in PROBS:
    seeds = (1, 2, 3, 4) + ((5, 6, 7, 8, 9, 10, 11, 12) if p.startswith("physiciansched6-2") else ())
    for s in seeds:
        gP[(p, s)] = [(p, "def", s), (p, "cand1114", s)]
balance(gP, estP, NP, "shards_P.json", 60)
