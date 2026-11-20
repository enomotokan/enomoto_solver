"""probework の対の比べの分担表 (scipslow_20261119/plan.py の写し): python plan.py NM NP → shards_M.json・shards_P.json
M: Mittelmann のうち prbscan.jsonl で PRBWORK が働きうる問題 (affected) × 種 1・2 × {def, pw}、600 秒。
P: MIPLIB の 58 問 × 種 1〜3 × {def, pw}、60 秒 (4 スレッド)。
同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。見積り: P は vshighs_20261117、M は vsscip_20261117 の def の時間 (+ 30 秒・8 秒)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parents[0]
sys.path.insert(0, str(A / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
ALT = "pw"


def affected(ev):
    """2 版目の PRBWORK が働きうる: 成果 0 (固定・締め付け 0) のまま「何も見つからない間の上限」(1.15e9) まで回ったプロービングがある
    (1 版目で走査したので、上限に届いた回は pwork ≥ 1.15e9)。再スタートが 60 秒・1 スレッドの走査で起きなくても、4 スレッド・600 秒では起きうるので含める"""
    return any(e[0] == "probe" and e[3] == 0 and e[4] == 0 and e[7] >= 1.15e9 for e in ev)


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
    k = s % 2
    return [(p, x, s) for x in c[k:] + c[:k]]


if __name__ == "__main__":
    NM, NP = int(sys.argv[1]), int(sys.argv[2])
    scan = [json.loads(l) for l in open(D / "prbscan.jsonl")]
    MPROBS = sorted(r["problem"] for r in scan if r["set"] == "M" and affected(r.get("events", [])))
    print("M の働く問題", len(MPROBS), MPROBS)
    TM = times(A / "vsscip_20261117/res_M_s*.jsonl", "def", 600)
    def estM(p, c): return TM.get(p, 600.0) + 30
    balance({(p, s): order(p, s, ["def", ALT]) for p in MPROBS for s in (1, 2)}, estM, NM, "shards_M.json", 600)
    TP = times(A / "vshighs_20261117/res_P_s*.jsonl", "def", 60)
    def estP(p, c): return TP.get(p, 48.0) + 8
    balance({(p, s): order(p, s, ["def", ALT]) for p in PROBS for s in (1, 2, 3)}, estP, NP, "shards_P.json", 60)
