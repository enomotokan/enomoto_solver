"""SCIP の手を 1 つずつ切って、根で閉じる決め手を切り分ける (vsscip_slow_20261118/tree2/scipab.py を広げた写し)。

使い方: python scipab.py 問題 設定[,設定...] [--tl 秒] [--seed N] [--log]
- 結果は scipab.jsonl に 1 行ずつ足す (同じ (問題, 設定, 種, tl) の行があれば飛ばす = 続きから)。
- --log で logs/scip_<問題>_<設定>.log にログ、logs/scip_<問題>_<設定>.stat に統計 (writeStatistics) を残す。
- 1 スレッド (SCIP は逐次)。設定の名前は apply() を見る。"""
import json, sys, time, argparse
from pathlib import Path

HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
OUT = HERE / "scipab.jsonl"

SEPAS = ['aggregation', 'clique', 'cmir', 'flowcover', 'gomory', 'gomorymi', 'strongcg', 'impliedbounds', 'knapsackcover',
         'mcf', 'mixing', 'oddcycle', 'zerohalf', 'disjunctive', 'rlt', 'intobj', 'lagromory', 'closecuts', 'interminor', 'minor']
PROPS = ['dualfix', 'genvbounds', 'obbt', 'probing', 'pseudoobj', 'redcost', 'rootredcost', 'symmetry', 'vbounds']
PRESOLS = ['boundshift', 'convertinttobin', 'domcol', 'dualagg', 'dualcomp', 'dualinfer', 'dualsparsify', 'gateextraction',
           'implics', 'implint', 'inttobinary', 'redvub', 'sparsify', 'stuffing', 'trivial', 'tworowbnd']
CONSSEPA = ['knapsack', 'linear', 'setppc', 'varbound', 'logicor']

def apply(m, name):
    from pyscipopt import SCIP_PARAMSETTING as PS
    if name == "def": return
    if name == "nosep": m.setSeparating(PS.OFF)
    elif name == "nopre": m.setPresolve(PS.OFF)
    elif name == "noprop": m.setParam("propagating/maxrounds", 0); m.setParam("propagating/maxroundsroot", 0)
    elif name == "noheur": m.setHeuristics(PS.OFF)
    elif name == "nosym": m.setParam("misc/usesymmetry", 0)
    elif name == "noconf": m.setParam("conflict/enable", False)
    elif name == "norestart": m.setParam("presolving/maxrestarts", 0)
    elif name == "nodual": m.setParam("misc/allowstrongdualreds", False); m.setParam("misc/allowweakdualreds", False)
    elif name == "pscost": m.setParam("branching/pscost/priority", 100000)
    elif name == "sep1": m.setParam("separating/maxroundsroot", 1)  # 根の分離を 1 巡だけ
    elif name.startswith("sep:"): m.setParam(f"separating/{name[4:]}/freq", -1)
    elif name.startswith("prop:"): m.setParam(f"propagating/{name[5:]}/freq", -1)
    elif name.startswith("presol:"): m.setParam(f"presolving/{name[7:]}/maxrounds", 0)
    elif name.startswith("csep:"): m.setParam(f"constraints/{name[5:]}/sepafreq", -1)
    elif name.startswith("p:"):  # p:名前=値;名前=値
        for kv in name[2:].split(";"):
            k, v = kv.split("=")
            cur = m.getParam(k)
            m.setParam(k, (v in ("1", "true", "True")) if isinstance(cur, bool) else type(cur)(v))
    else:
        raise SystemExit(f"未知の設定 {name}")

def expand(sets):
    out = []
    for s in sets.split(","):
        if s == "GROUPS": out += ["def", "nosep", "nopre", "noprop", "noheur", "nosym", "noconf", "norestart", "sep1"]
        elif s == "SEPAS": out += ["sep:" + x for x in SEPAS] + ["csep:" + x for x in CONSSEPA]
        elif s == "PROPS": out += ["prop:" + x for x in PROPS]
        elif s == "PRESOLS": out += ["presol:" + x for x in PRESOLS]
        else: out.append(s)
    return out

def done():
    d = set()
    if OUT.exists():
        for l in OUT.open():
            r = json.loads(l); d.add((r["problem"], r["set"], r["seed"], r["tl"]))
    return d

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("prob"); ap.add_argument("sets"); ap.add_argument("--tl", type=float, default=300); ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--log", action="store_true")
    a = ap.parse_args()
    import pyscipopt
    mps = WT / ".miplib_cache" / f"{a.prob}.mps"
    dn = done()
    for s in expand(a.sets):
        if (a.prob, s, a.seed, a.tl) in dn: continue
        m = pyscipopt.Model()
        tag = s.replace(":", "_").replace("/", "-").replace(";", "+").replace("=", "")
        m.hideOutput()
        if a.log: m.setLogfile(str(HERE / "logs" / f"scip_{a.prob}_{tag}.log"))
        m.setParam("limits/time", a.tl); m.setParam("limits/gap", 0.0)
        if a.seed: m.setParam("randomization/randomseedshift", a.seed)
        apply(m, s)
        m.readProblem(str(mps))
        t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
        has = m.getNSols() > 0
        r = {"problem": a.prob, "set": s, "seed": a.seed, "tl": a.tl, "status": m.getStatus(), "time": round(t, 2),
             "objective": m.getObjVal() if has else None, "bound": m.getDualbound(), "root_bound": m.getDualboundRoot(),
             "nodes": m.getNTotalNodes(), "lpiters": m.getNLPIterations(), "ncuts_applied": m.getNCutsApplied(),
             "nruns": m.getNRuns()}
        if a.log: m.writeStatistics(str(HERE / "logs" / f"scip_{a.prob}_{tag}.stat"))
        print(json.dumps(r), flush=True)
        with OUT.open("a") as f: f.write(json.dumps(r) + "\n")

if __name__ == "__main__":
    main()
