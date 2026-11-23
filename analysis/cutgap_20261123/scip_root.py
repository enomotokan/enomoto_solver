"""SCIP (pyscipopt 6.3.0 = SCIP 10.0.2) の根の下界の推移を取る: python scip_root.py 問題 [切る分離器,... | -] [設定...]
- limits/nodes = 1 (根だけ)、limits/time = 600、逐次、種の既定。
- ログ (logs/scip_<問題>_<名札>.log、verblevel 4) の表の行 (時刻・ノード・カット数・分離ラウンド・下界) を拾う。first_lp は最初の行、after_cuts は根の最後から 2 番目の行 (最後は強分枝の後のことが多い)。
- 切る分離器: separating/<名>/freq = -1。"cons:<名>" は constraints/<名>/sepafreq = -1。"allsep" は separating/maxrounds・maxroundsroot = 0 (カットを全部切る)。
- 設定の追加は 名=値 (例 presolving/maxrounds=0)。
- 出力: 1 行の JSON (stdout)、統計は logs/scip_<問題>_<名札>.stat。"""
import json, os, sys, time
from pathlib import Path
import pyscipopt
from pyscipopt import Eventhdlr, SCIP_EVENTTYPE
D = Path(__file__).resolve().parent; WT = D.parents[1]
p = sys.argv[1]; off = sys.argv[2] if len(sys.argv) > 2 else "-"; extra = sys.argv[3:]
tag = os.environ.get("TAG", off.replace(",", "+") + ("_" + "_".join(e.replace("/", ".") for e in extra) if extra else ""))
class H(Eventhdlr):
    def eventinit(self): self.model.catchEvent(SCIP_EVENTTYPE.LPSOLVED, self)
    def eventexit(self): self.model.dropEvent(SCIP_EVENTTYPE.LPSOLVED, self)
    def eventexec(self, ev):
        m = self.model
        if m.getNNodes() > 1: return
        try: v = m.getLPObjVal()
        except Exception: v = None
        self.rows.append([round(time.perf_counter() - T0, 2), v, m.getNSepaRounds(), m.getNLPRows()])
m = pyscipopt.Model(); (D / "logs").mkdir(exist_ok=True); LOG = D / "logs" / f"scip_{p}_{tag}.log"; m.setLogfile(str(LOG)); m.hideOutput()
m.hideOutput(False); m.setParam("display/verblevel", 4)
m.setParam("limits/time", 600); m.setParam("limits/nodes", 1)
for o in ([] if off == "-" else off.split(",")):
    if o == "allsep": m.setParam("separating/maxroundsroot", 0); m.setParam("separating/maxrounds", 0)
    elif o.startswith("cons:"): m.setParam(f"constraints/{o[5:]}/sepafreq", -1)
    else: m.setParam(f"separating/{o}/freq", -1)
for e in extra:
    k, v = e.split("=", 1); cur = m.getParam(k)
    m.setParam(k, type(cur)(v) if not isinstance(cur, bool) else v.lower() in ("1", "true"))
m.readProblem(str(WT / ".miplib_cache" / f"{p}.mps"))
T0 = time.perf_counter(); m.optimize(); t = time.perf_counter() - T0
(D / "logs").mkdir(exist_ok=True); st = D / "logs" / f"scip_{p}_{tag}.stat"; m.writeStatistics(str(st))
txt = st.read_text(); sepa = {}
# Separators の表: 名前ごとの Applied (LP に入ったカット) を拾う
sec = txt.split("Separators         :")[1].split("\n\n")[0].split("Pricers")[0] if "Separators         :" in txt else ""
for l in sec.splitlines()[1:]:
    f = [x.strip() for x in l.split(":")]
    if len(f) == 2 and f[1]:
        nums = f[1].split()
        sepa[f[0]] = nums
tab = [l.split("|") for l in LOG.read_text().splitlines() if l.count("|") >= 15 and not l.lstrip().startswith("time")]
tab = [[c.strip() for c in r] for r in tab]
rr = [(float(r[0].lstrip("*abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ ").rstrip("s")), int(r[1]), int(r[10]) if r[10].isdigit() else r[10], int(r[11]) if r[11].isdigit() else r[11], float(r[14]) if r[14] not in ("--",) else None) for r in tab]
row = {"problem": p, "off": off, "extra": extra, "status": m.getStatus(), "root_dual": m.getDualbound(), "primal": m.getPrimalbound() if m.getNSols() else None,
       "first_lp": rr[0][4] if rr else None, "after_cuts": [x for x in rr if x[1] == 1 and x[0] >= 0][-2][4] if len(rr) > 1 else None, "table": rr, "time": round(t, 2),
       "presolved": [m.getNVars(True), m.getNConss(True)] if False else None}
try: row["presolved"] = [m.getNVars(), m.getNConss()]
except Exception: pass
print(json.dumps(row))
with open(D / "scip.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
