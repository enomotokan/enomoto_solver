# MIPLIB の .sol から整数変数を固定し、連続部分を HiGHS の LP で解き直して精密な最適値を作る
import sys, gzip, json, math, urllib.request, highspy, numpy as np
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_miplib_benchmark as r
S = Path(sys.argv[1]); cache = Path(__file__).resolve().parents[2] / '.miplib_cache'
solu = {}
for l in open(S/'miplib.solu'):
    p = l.split()
    if len(p) > 2: solu[p[1]] = float(p[2])
out = {}
for n in r.PROBLEMS:
    sp = S/'sol'/f'{n}.sol'
    if not sp.exists():
        try:
            with urllib.request.urlopen(f'https://miplib.zib.de/downloads/solutions/{n}/1/{n}.sol.gz', timeout=60) as u:
                sp.write_bytes(gzip.decompress(u.read()))
        except Exception as e:
            print(n, 'nosol', e); out[n] = {'solu': solu.get(n), 'ref': solu.get(n)}; continue
    vals = {}
    for l in sp.read_text().splitlines():
        p = l.split()
        if len(p) >= 2 and not l.startswith('#') and not l.startswith('=') and p[0] != 'objective':
            try: vals[p[0]] = float(p[1])
            except: pass
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(str(cache/f'{n}.mps'))
    lp = h.getLp(); nc = lp.num_col_
    names = list(lp.col_names_); integ = list(lp.integrality_)
    lo = np.array(lp.col_lower_); hi = np.array(lp.col_upper_)
    x = np.array([vals.get(nm, 0.0) for nm in names])
    hasc = False
    for j in range(nc):
        if integ and int(integ[j]) != 0:
            v = float(round(x[j])); lo[j] = hi[j] = v
        else: hasc = True
    h.changeColsIntegrality(nc, np.arange(nc, dtype=np.int32), np.zeros(nc, dtype=np.uint8))
    h.changeColsBounds(nc, np.arange(nc, dtype=np.int32), lo, hi)
    h.setOptionValue('primal_feasibility_tolerance', 1e-10); h.setOptionValue('dual_feasibility_tolerance', 1e-10)
    h.run()
    st = h.modelStatusToString(h.getModelStatus())
    xs = np.array(h.getSolution().col_value)
    c = list(lp.col_cost_)
    ref = math.fsum([c[j]*xs[j] for j in range(nc)]) + lp.offset_
    out[n] = {'solu': solu.get(n), 'ref': ref, 'status': st}
    print(f'{n:20s} {st:10s} ref={ref!r} solu={solu.get(n)} rel={abs(ref-solu.get(n,ref))/max(1,abs(ref)):.1e}', flush=True)
json.dump(out, open(S/'ref.json','w'), indent=1)
