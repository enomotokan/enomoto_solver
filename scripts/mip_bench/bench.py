"""ours だけを 1 問 1 サブプロセスで解く。-j 並列。結果 JSON に目的値・時間・ref との相対誤差・解の違反量。"""
import sys, json, math, os, subprocess, time, argparse
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_miplib_benchmark as r
REPO = Path(__file__).resolve().parents[2]
S = Path(os.environ.get('MIP_BENCH_DIR', Path(__file__).parent / 'work'))
CACHE = REPO / '.miplib_cache'

def worker(mps, tl, gap, threads=1, nodes=None):
    from enomoto_solver import benchmark_highs as bh
    import numpy as np
    _, lp = bh._load_lp(Path(mps))
    m, _, _ = bh._build_our_model(lp)
    import resource
    ru0 = resource.getrusage(resource.RUSAGE_SELF)
    t = time.perf_counter()
    res = m.solve(None, False, tl, gap, nodes, threads)
    t = time.perf_counter() - t
    ru1 = resource.getrusage(resource.RUSAGE_SELF)
    cpu = (ru1.ru_utime - ru0.ru_utime) + (ru1.ru_stime - ru0.ru_stime)
    out = {k: res.get(k) for k in ('status', 'objective', 'best_bound', 'nodes', 'root_bound', 'first_solution_time', 'first_solution_objective')}
    out['time'] = t
    out['cpu'] = cpu
    x = res['x']
    if x is not None:
        x = np.array(x)
        c = list(lp.col_cost_)
        out['obj_x'] = math.fsum(c[j]*x[j] for j in range(len(c))) + lp.offset_
        lo = np.array(lp.col_lower_); hi = np.array(lp.col_upper_)
        out['bnd_viol'] = float(max(0, np.max(np.maximum(lo - x, x - hi))))
        integ = np.array([int(v) for v in lp.integrality_]) if len(lp.integrality_) else np.zeros(len(x))
        out['int_viol'] = float(np.max(np.abs(x - np.round(x)) * (integ != 0))) if len(x) else 0.0
        am = lp.a_matrix_; st, ix, va = np.array(am.start_), np.array(am.index_), np.array(am.value_)
        ax = np.zeros(lp.num_row_)
        for j in range(lp.num_col_):
            s, e = st[j], st[j+1]
            ax[ix[s:e]] += va[s:e] * x[j]
        rl = np.array(lp.row_lower_); ru = np.array(lp.row_upper_)
        out['row_viol'] = float(max(0, np.max(np.maximum(rl - ax, ax - ru)))) if lp.num_row_ else 0.0
    print(json.dumps(out))

def run(n, tl, gap, extra_env):
    cmd = [sys.executable, __file__, '--worker', str(CACHE/f'{n}.mps'), '--tl', str(tl), '--gap', str(gap), '--threads', str(THREADS)]
    if NODES: cmd += ['--nodes', str(NODES)]
    env = dict(os.environ, **extra_env)
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=tl*1.5+60, env=env)
    except subprocess.TimeoutExpired:
        return {'status': 'killed', 'time': None}
    for line in reversed(p.stdout.splitlines()):
        if line.startswith('{'): return json.loads(line)
    return {'status': f'crash rc={p.returncode}', 'time': None, 'stderr': p.stderr[-1500:]}

def solved(o): return o.get('status') == 'optimal'

def primal_gap(obj, ref):
    """主ギャップ (SCIP/MIPLIB の定義): 解なし・符号が逆なら 1、それ以外は |obj - ref| / max(|obj|, |ref|)。"""
    if obj is None or ref is None: return 1.0
    if abs(obj - ref) <= 1e-9 * max(1.0, abs(ref)): return 0.0
    if obj * ref < 0: return 1.0
    return abs(obj - ref) / max(abs(obj), abs(ref))

def root_gap(rb, ref):
    """根の双対ギャップ |ref - 根の下界| / max(|ref|, |根の下界|, 1) (根の下界がなければ 1)。"""
    if rb is None or ref is None: return 1.0
    return min(1.0, abs(ref - rb) / max(abs(ref), abs(rb), 1.0))

def ref_value(ref, n):
    e = ref.get(n) or {}
    return e.get('solu') if e.get('solu') is not None else e.get('ref')

def summarize(res, tl, ref):
    ts = []; errs = []
    for n in r.PROBLEMS:
        if n not in res: continue
        o = res[n]
        ts.append(o['time'] if solved(o) and o['time'] is not None else tl)
        if solved(o) and n in ref and ref[n].get('status') == 'Optimal':
            rv = ref[n]['ref']; errs.append((abs(o['objective']-rv)/max(1.0,abs(rv)), n))
    sgm = math.exp(sum(math.log(t+10) for t in ts)/len(ts)) - 10
    return sgm, sum(solved(res[n]) for n in res), sorted(errs, reverse=True)

def quality(res, tl, ref):
    """時間上限での主ギャップの和、根のギャップの和、最初の暫定解の時刻のずらし幾何平均 (解なしは上限)。"""
    pg = rg = 0.0; ft = []
    for n in r.PROBLEMS:
        if n not in res: continue
        o = res[n]; rv = ref_value(ref, n)
        pg += primal_gap(o.get('objective'), rv)
        rg += root_gap(o.get('root_bound'), rv)
        t = o.get('first_solution_time')
        ft.append(tl if t is None else min(t, tl))
    fsg = math.exp(sum(math.log(t+1) for t in ft)/len(ft)) - 1 if ft else float('nan')
    return pg, rg, fsg

if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--worker'); ap.add_argument('--tl', type=float, default=90); ap.add_argument('--gap', type=float, default=-1)
    ap.add_argument('--out'); ap.add_argument('-j', type=int, default=4); ap.add_argument('--only', nargs='*')
    ap.add_argument('--env', nargs='*', default=[]); ap.add_argument('--threads', type=int, default=1)
    ap.add_argument('--nodes', type=int, help='ノード数上限 (1 なら根だけ)')
    a = ap.parse_args()
    gap = None if a.gap is not None and a.gap < 0 else a.gap
    THREADS = a.threads; NODES = a.nodes
    if a.worker: worker(a.worker, a.tl, gap, a.threads, a.nodes); sys.exit()
    env = dict(e.split('=', 1) for e in a.env)
    names = a.only or r.PROBLEMS
    res = {}
    outp = Path(a.out)
    if outp.exists(): res = json.loads(outp.read_text())
    todo = [n for n in names if n not in res]
    with ThreadPoolExecutor(a.j) as ex:
        for n, o in zip(todo, ex.map(lambda n: run(n, a.tl, a.gap, env), todo)):
            res[n] = o; outp.write_text(json.dumps(res, indent=1))
            print(f"{n:20s} {o['status']:>12s} t={o.get('time')} obj={o.get('objective')!r} objx={o.get('obj_x')!r} viol={o.get('row_viol')}", flush=True)
    ref = json.loads((S/'ref.json').read_text())
    sgm, ns, errs = summarize(res, a.tl, ref)
    pg, rg, fsg = quality(res, a.tl, ref)
    print(f'SGM={sgm:.3f} solved={ns} primal_gap_sum={pg:.4f} root_gap_sum={rg:.4f} first_sol_sgm1={fsg:.2f}')
    for e, n in errs[:15]: print(f'  relerr {n:20s} {e:.2e}')
