"""SCIP 10 (pyscipopt) で前処理だけを行い、前処理器ごとの統計 (消した列・行など) を出す。使い方: python scip_pre.py MPS [名前=値 ...]"""
import sys, time
import pyscipopt
m = pyscipopt.Model()
m.hideOutput()
for kv in sys.argv[2:]:
    k, v = kv.split("=", 1)
    try: v = int(v)
    except ValueError:
        try: v = float(v)
        except ValueError: pass
    m.setParam(k, v)
m.readProblem(sys.argv[1])
t = time.perf_counter(); m.presolve(); t = time.perf_counter() - t
print(f"presolve {t:.2f}s vars {m.getNVars()} conss {m.getNConss()}")
m.writeStatistics("/dev/stdout") if hasattr(m, "writeStatistics") else m.printStatistics()
