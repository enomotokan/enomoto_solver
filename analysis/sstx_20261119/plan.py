"""SSTX の分担表: python plan.py NM NP → shards_M.json・shards_P.json
対象は analysis/hang_20261117 の symscan_M.txt・symscan_P.txt で生成元が 1 つ以上の問題 (M 19 問・P 21 問)。
× 種 1〜3 × {def, sst, sstx}、M 600 秒・P 60 秒、4 スレッド。同じ (問題, 種) の 3 組は同じ分担で続けて解き、順は種で回す。
見積り: hang_20261117 の MSST・PSST と vshighs_20261117 の時間の平均 (+ 30 秒・8 秒)。"""
import json, glob, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
H = D.parent / "hang_20261117"
VS = D.parent / "vshighs_20261117"
NM, NP = int(sys.argv[1]), int(sys.argv[2])
CF = ["def", "sst", "sstx"]
def gens(f):
    out = []
    for l in open(f):
        m = re.match(r"^([^:]+): MIP: symmetry: (\d+) generators", l)
        if m and int(m.group(2)) > 0: out.append(m.group(1))
    return sorted(set(out))
def times(pats, cap):
    T = defaultdict(list)
    for pat in pats:
        for f in glob.glob(str(pat)):
            for l in open(f):
                r = json.loads(l)
                if r.get("cfg") not in ("def", "sst"): continue
                t = (r.get("result") or {}).get("time") or cap
                T[r["problem"]].append(min(t, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(probs, est, n, out, tl):
    groups = {(p, s): [(p, CF[(i + s) % 3], s) for i in range(3)] for p in probs for s in (1, 2, 3)}
    gl = sorted(groups.items(), key=lambda kv: -3 * est(kv[0][0]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += 3 * est(p)
    (D / out).write_text(json.dumps({"problems": probs, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(probs), "問", len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
MP, PP = gens(H / "symscan_M.txt"), gens(H / "symscan_P.txt")
TM = times([H / "res_MSST_s*.jsonl", VS / "res_M_s*.jsonl"], 600)
TP = times([H / "res_PSST_s*.jsonl", VS / "res_P_s*.jsonl"], 60)
balance(MP, lambda p: TM.get(p, 600.0) + 30, NM, "shards_M.json", 600)
balance(PP, lambda p: TP.get(p, 48.0) + 8, NP, "shards_P.json", 60)
