"""群 2 (木の効率): SCIP の機能を 1 つずつ切って、SCIP が速い決め手を切り分ける。

使い方: python scipab.py 問題 設定[,設定...] [--tl 秒] [--seed N]
結果は scipab.jsonl に 1 行ずつ足す。設定の名前は SETS を見る。"""
import json, os, sys, time, argparse
from pathlib import Path

HERE = Path(__file__).resolve().parent
WT = HERE.parents[2]

def apply(m, name):
    from pyscipopt import SCIP_PARAMSETTING as PS
    if name == "def":
        return
    if name == "nosep":  m.setSeparating(PS.OFF)
    elif name == "nosepnode":  m.setParam("separating/maxrounds", 0)  # 根だけ分離
    elif name == "noprop":  m.setParam("propagating/maxrounds", 0); m.setParam("propagating/maxroundsroot", 0)
    elif name == "nopre":  m.setPresolve(PS.OFF)
    elif name == "noheur":  m.setHeuristics(PS.OFF)
    elif name == "nosym":  m.setParam("misc/usesymmetry", 0)
    elif name == "noconf":  m.setParam("conflict/enable", False)
    elif name == "norestart":  m.setParam("presolving/maxrestarts", 0)
    elif name == "pscost":  m.setParam("branching/pscost/priority", 100000)  # 強分枝なし
    elif name == "nostrong":  # relpscost のまま強分枝の回数を絞る
        m.setParam("branching/relpscost/maxreliable", 0.0); m.setParam("branching/relpscost/minreliable", 0.0)
    elif name == "dfs":  m.setParam("nodeselection/dfs/stdpriority", 1000000)
    elif name == "nocmir":  m.setParam("separating/aggregation/freq", -1)
    elif name == "nogom":  m.setParam("separating/gomory/freq", -1)
    elif name == "noclq":  m.setParam("separating/clique/freq", -1)
    elif name == "nozh":  m.setParam("separating/zerohalf/freq", -1)
    elif name == "nomcf":  m.setParam("separating/mcf/freq", -1)
    elif name == "noimpl":  m.setParam("separating/impliedbounds/freq", -1)
    elif name == "nodual":  m.setParam("misc/allowstrongdualreds", False); m.setParam("misc/allowweakdualreds", False)
    elif name.startswith("p:"):  # p:名前=値;名前=値
        for kv in name[2:].split(";"):
            k, v = kv.split("=")
            cur = m.getParam(k)
            m.setParam(k, type(cur)(v) if not isinstance(cur, bool) else v in ("1", "true", "True"))
    else:
        raise SystemExit(f"未知の設定 {name}")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("prob"); ap.add_argument("sets"); ap.add_argument("--tl", type=float, default=600); ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--log", action="store_true")
    a = ap.parse_args()
    import pyscipopt
    mps = WT / ".miplib_cache" / f"{a.prob}.mps"
    for s in a.sets.split(","):
        m = pyscipopt.Model()
        if not a.log: m.hideOutput()
        m.setParam("limits/time", a.tl); m.setParam("limits/gap", 0.0)
        if a.seed: m.setParam("randomization/randomseedshift", a.seed)
        apply(m, s)
        m.readProblem(str(mps))
        t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
        has = m.getNSols() > 0
        r = {"problem": a.prob, "set": s, "seed": a.seed, "tl": a.tl, "status": m.getStatus(), "time": round(t, 2),
             "objective": m.getObjVal() if has else None, "bound": m.getDualbound(), "root_bound": m.getDualboundRoot(),
             "nodes": m.getNTotalNodes(), "lpiters": m.getNLPIterations(), "nsepa_cuts": m.getNCutsApplied(),
             "first_sol_time": None}
        print(json.dumps(r), flush=True)
        with open(HERE / "scipab.jsonl", "a") as f: f.write(json.dumps(r) + "\n")

if __name__ == "__main__":
    main()
