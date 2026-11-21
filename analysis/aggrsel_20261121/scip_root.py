"""SCIP (pyscipopt) の根だけ: python scip_root.py 並べる数 [札=param:value,... ...]。
問題 PROBS × 札 (VARS) を 1 スレッド・limits/nodes=1・再スタートなし (presolving/maxrestarts=0) で解き、根の下界と分離器ごとの統計
(Separators の表: 呼んだ回数・見つけたカット・LP に入ったカット) を scip_root.jsonl に足す。統計の全文は logs/scip_<問題>_<札>.stats。"""
import json, sys, re
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
PROBS = "unitcal_7 neos-911970 neos-1582420 mc11 p200x1188c beasleyC3".split()
BASE = {"limits/nodes": 1, "presolving/maxrestarts": 0, "limits/time": 900.0, "randomization/randomseedshift": 0}
VARS = {
    "base": {},
    "noaggr": {"separating/aggregation/freq": -1},
    "aggr0": {"separating/aggregation/maxaggrsroot": 0},
    "aggr1": {"separating/aggregation/maxaggrsroot": 1},
    "aggr3": {"separating/aggregation/maxaggrsroot": 3},
    "nodens": {"separating/aggregation/maxrowdensity": 1.0, "separating/aggregation/maxaggdensity": 1.0},
    "anyslack": {"separating/aggregation/maxslackroot": 1e20},
    "fails5": {"separating/aggregation/maxfailsroot": 5},
}
def sep_table(txt):
    out = {}; on = False
    for l in txt.splitlines():
        if l.startswith("Separators"): on = True; continue
        if on:
            if not l.startswith("  "): break
            m = re.match(r"\s+([\w-]+)\s*:\s+(.*)", l)
            if m:
                f = m.group(2).split()
                out[m.group(1)] = f
    return out
def one(job):
    p, tag = job
    import pyscipopt
    m = pyscipopt.Model(); m.hideOutput()
    m.readProblem(str(WT / ".miplib_cache" / f"{p}.mps"))
    for k, v in {**BASE, **VARS[tag]}.items(): m.setParam(k, v)
    m.optimize()
    st = D / "logs" / f"scip_{p}_{tag}.stats"
    m.writeStatistics(str(st))
    txt = st.read_text()
    r = {"problem": p, "tag": tag, "status": m.getStatus(), "root_bound": m.getDualbound(), "time": m.getSolvingTime(),
         "lp_iters": m.getNLPIterations(), "sep": sep_table(txt)}
    m2 = re.search(r"Root Node\s*:\s*\n\s*First LP value\s*:\s*(\S+)", txt)
    if m2: r["first_lp"] = float(m2.group(1))
    return r
if __name__ == "__main__":
    nj = int(sys.argv[1]) if len(sys.argv) > 1 else 3
    tags = sys.argv[2:] or list(VARS)
    out = D / "scip_root.jsonl"
    done = {(json.loads(l)["problem"], json.loads(l)["tag"]) for l in open(out)} if out.exists() else set()
    (D / "logs").mkdir(exist_ok=True)
    jobs = [(p, t) for t in tags for p in PROBS if (p, t) not in done]
    with ProcessPoolExecutor(nj) as ex:
        for r in ex.map(one, jobs):
            with open(out, "a") as f: f.write(json.dumps(r) + "\n")
            print(r["problem"], r["tag"], r["root_bound"], round(r["time"], 1), flush=True)
