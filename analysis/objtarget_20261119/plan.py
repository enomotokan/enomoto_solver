"""OBJTARGET の計測の分担表: python plan.py NM NP [試作] → shards_M<札>.json・shards_P<札>.json (試作は既定 l。p なら M に解なしの 13 問も足す)
ML: Mittelmann の解探しが弱い「解はあるが悪い」32 問 (README 1) + 600 秒で 1 割の時点に良い解のあった 6 問 + 下界側が弱い 8 問 × 種 1・2 × {def, l}、600 秒。
PL: MIPLIB の 58 問 × 種 1〜3 × {def, l}、60 秒。4 スレッド。同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。
見積り: 最終計測 (analysis/final_20261118) の def の時間 (+ 30 秒・8 秒)。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
ALT = sys.argv[3] if len(sys.argv) > 3 else "l"  # 試作の設定 (cfg.py)。札は大文字 (l → ML・PL、p → MP・PP)
SUF = ALT.upper()
POOR = ["brazil3", "markshare2", "trento1", "momentum1", "mad", "comp21-2idx", "neos-5195221-niemur", "neos-4300652-rahue", "lectsched-5-obj",
        "neos-5107597-kakapo", "glass4", "neos-4763324-toguru", "opm2-z10-s4", "rocII-5-11", "sorrell3", "nexp-150-20-8-5", "traininstance2",
        "neos-4387871-tavua", "sing326", "neos-873061", "rococoB10-011000", "cvs16r128-89", "neos-5188808-nattai", "neos-3656078-kumeu", "leo2",
        "csched007", "neos-933966", "sp97ar", "uct-subprob", "neos-3024952-loue", "b1c1s1", "blp-ic98"]
GOOD10 = ["atlanta-ip", "supportcase18", "radiationm40-10-02", "leo1", "thor50dday", "supportcase42"]
BOUND = ["gen-ip002", "gen-ip054", "assign1-5-8", "30n20b8", "reblock115", "ran14x18-disj-8", "bppc4-08", "neos-662469"]
# 解なしの 13 問 (README 1。PDLP の点から最初の解を狙う p の対象)
NOSOL = ["academictimetablesmall", "satellites2-60-fs", "satellites2-40", "rail01", "neos-3216931-puriri", "dws008-01", "cmflsp50-24-8-8",
         "ns1952667", "neos-1354092", "bnatt400", "neos-2746589-doon", "germanrr", "peg-solitaire-a3"]
MPROBS = POOR + GOOD10 + BOUND + (NOSOL if ALT == "p" else [])
def times(pat, cap):
    T = {}
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r.get("cfg") != "def": continue
            t = (r.get("result") or {}).get("time") or cap
            T.setdefault(r["problem"], []).append(min(t, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(probs, seeds, est, n, out, tl):
    groups = {(p, s): [(p, c, s) for c in (["def", ALT] if s % 2 else [ALT, "def"])] for p in probs for s in seeds}
    gl = sorted(groups.items(), key=lambda kv: -2 * est(kv[0][0]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += 2 * est(p)
    (D / out).write_text(json.dumps({"problems": probs, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(probs), "問", len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
F = D.parent / "final_20261118"
TM = times(F / "res_M_s*.jsonl", 600)
TP = times(F / "res_P_s*.jsonl", 60)
balance(MPROBS, (1, 2), lambda p: TM.get(p, 600.0) + 30, NM, f"shards_M{SUF}.json", 600)
balance(list(PROBS), (1, 2, 3), lambda p: TP.get(p, 48.0) + 8, NP, f"shards_P{SUF}.json", 60)
