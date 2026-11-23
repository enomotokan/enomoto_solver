"""SCIP の前処理後の問題 (変換後の問題) を presolved_scip/<問題>.mps に書く: python scip_presolved.py 問題"""
import sys
from pathlib import Path
import pyscipopt
D = Path(__file__).resolve().parent; WT = D.parents[1]; p = sys.argv[1]
m = pyscipopt.Model(); m.hideOutput(); m.readProblem(str(WT / ".miplib_cache" / f"{p}.mps")); m.presolve()
(D / "presolved_scip").mkdir(exist_ok=True); m.writeProblem(str(D / "presolved_scip" / f"{p}.mps"), trans=True)
print(p, m.getNVars(), m.getNConss(), "offset", m.getTransObjoffset() if hasattr(m, "getTransObjoffset") else None)
