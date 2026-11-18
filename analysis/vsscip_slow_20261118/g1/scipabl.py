"""群 1 の切り分け: SCIP 10.0.2 のパラメータを 1 つずつ切って、どれが決め手かを見る。
python scipabl.py 問題 [秒] [設定,設定,...] → scipabl.jsonl に足す。設定は ABL の名前 (既定は全部)。
SCIP は逐次。種は既定 (randomseedshift 0)。出力は抑える。"""
import json, sys, time
from pathlib import Path
import pyscipopt
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = pyscipopt.SCIP_PARAMSETTING
ABL = {
    "base": lambda m: None,
    "nosym": lambda m: m.setParam("misc/usesymmetry", 0),
    "noorbitope": lambda m: m.setParam("propagating/symmetry/detectorbitopes", False),
    "noconf": lambda m: m.setParam("conflict/enable", False),
    "nopresol": lambda m: m.setPresolve(P.OFF),
    "nocuts": lambda m: m.setSeparating(P.OFF),
    "noheur": lambda m: m.setHeuristics(P.OFF),
    "noprop": lambda m: m.setParam("propagating/maxrounds", 0) or m.setParam("propagating/maxroundsroot", 0),
    "noclique": lambda m: m.setParam("separating/clique/freq", -1),
    "norestart": lambda m: m.setParam("presolving/maxrestarts", 0),
    "pscost": lambda m: m.setParam("branching/pscost/priority", 100000),  # 信頼度つきの強い分枝の代わりに擬費用だけ
    "nodual": lambda m: m.setParam("misc/allowstrongdualreds", False) or m.setParam("misc/allowweakdualreds", False),
    # 2 段目 (群 1 の結果を受けて): 対称性の種類 (misc/usesymmetry のビット 1 = 多面体 (symresack・オービトープ)、2 = 軌道の縮小、4 = SST)、
    # 前処理の各部品、制約ごとの双対の前処理
    "sym1": lambda m: m.setParam("misc/usesymmetry", 1),
    "sym2": lambda m: m.setParam("misc/usesymmetry", 2),
    "sym4": lambda m: m.setParam("misc/usesymmetry", 4),
    "sym3": lambda m: m.setParam("misc/usesymmetry", 3),
    "nodynprop": lambda m: m.setParam("propagating/symmetry/usedynamicprop", False),
    "nodualcons": lambda m: [m.setParam(f"constraints/{c}/{n}", False) for c, n in (("linear", "dualpresolving"), ("knapsack", "dualpresolving"), ("logicor", "dualpresolving"), ("setppc", "dualpresolving"))],
    "nodual_linear": lambda m: m.setParam("constraints/linear/dualpresolving", False),
    "nodual_knapsack": lambda m: m.setParam("constraints/knapsack/dualpresolving", False),
    "nodual_logicor": lambda m: m.setParam("constraints/logicor/dualpresolving", False),
    "nodual_setppc": lambda m: m.setParam("constraints/setppc/dualpresolving", False),
    "nolinpresolpair": lambda m: m.setParam("constraints/linear/presolpairwise", False),
    "nosetppcclq": lambda m: m.setParam("constraints/setppc/cliquelifting", False),
    "noprobing": lambda m: m.setParam("propagating/probing/maxprerounds", 0),
    "noconflag": lambda m: m.setParam("constraints/linear/aggregatevariables", False),
}
for _p in ['boundshift', 'convertinttobin', 'domcol', 'dualagg', 'dualcomp', 'dualinfer', 'dualsparsify', 'gateextraction', 'implics', 'implint',
           'inttobinary', 'redvub', 'sparsify', 'stuffing', 'trivial', 'tworowbnd']:
    ABL["no_" + _p] = (lambda q: lambda m: m.setParam(f"presolving/{q}/maxrounds", 0))(_p)
prob = sys.argv[1]; tl = float(sys.argv[2]) if len(sys.argv) > 2 else 120
names = sys.argv[3].split(",") if len(sys.argv) > 3 else list(ABL)
mps = rb.fetch(prob, WT / ".miplib_cache")
for n in names:
    m = pyscipopt.Model(); m.hideOutput(); m.setParam("limits/time", tl); m.setParam("limits/gap", 0.0)
    ABL[n](m); m.readProblem(str(mps))
    t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
    r = {"problem": prob, "abl": n, "status": m.getStatus(), "time": round(t, 2), "nodes": m.getNTotalNodes(),
         "obj": m.getObjVal() if m.getNSols() else None, "bound": m.getDualbound()}
    with open(D / "scipabl.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
    print(json.dumps(r), flush=True)
