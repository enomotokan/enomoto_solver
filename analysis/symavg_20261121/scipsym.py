# SCIP の対称性の検出の結果 (生成元の数・群の大きさ log10・成分の扱い) を前処理だけで取る: python scipsym.py <mps>...
import sys, re, os, tempfile
from pyscipopt import Model
for mps in sys.argv[1:]:
    m = Model(); m.hideOutput()
    m.setParam('propagating/symmetry/dispsyminfo', True); m.setParam('limits/time', 120)
    log = tempfile.mktemp(); m.setLogfile(log); m.hideOutput(False)
    m.readProblem(mps); m.presolve()
    t = open(log).read(); os.unlink(log)
    g = re.findall(r'symmetry computation finished: (\d+) generators found.*?log10 of symmetry group size: ([\d.]+)', t)
    use = re.findall(r'  use .*|orbitopal reduction: .*|orbital reduction: .*|packing/partitioning.*', t)
    print(os.path.basename(mps), 'gens/log10', g, '| vars', m.getNVars(), '|', '; '.join(sorted(set(u.strip() for u in use)))[:300], flush=True)
