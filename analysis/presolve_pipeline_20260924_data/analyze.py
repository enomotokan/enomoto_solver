import json, sys, statistics, math
from collections import defaultdict

path = sys.argv[1] if len(sys.argv) > 1 else "full_ps.json"
d = json.load(open(path))

def agg(runs, key, how=min):
    vals = [r["pipe"].get(key, 0.0) for r in runs]
    return how(vals) if vals else 0.0

rows = []
step_tot = defaultdict(float)      # us summed over all problems
step_tot_light = defaultdict(float)
step_cnt = defaultdict(int)
light_solve = 0.0; all_solve = 0.0
for p in d:
    runs = p["prof"]
    g = lambda k: agg(runs, k)
    tot = g("solve_mip")
    pre = g("presolve_total")
    bag = g("build_a_g")
    bsf = g("build_std_form_presolved")
    stdform = bsf - g("run_extended")  # build_a_g + rows + freeze + shift/compaction
    hash_extra = g("run_extended") - pre
    setup = g("ext_setup_total")
    lu0 = g("ext_initial_lu")
    main = g("ext_main")
    fin = g("ext_finish")
    ext = g("ext_total")
    post = g("unscale_result") + g("drop_std")
    other = tot - bsf - ext - post
    it = runs[-1]["pipe_cnt"].get("ext_iters", 0)
    rounds = runs[-1]["pipe_cnt"].get("rounds", 0)
    sz = runs[-1]
    r = dict(name=p["name"], n=p.get("n"), m=p.get("m"), nnz=p.get("nnz"),
             n_out=sz.get("n_vars_out"), m_out=sz.get("n_rows_out"),
             med=p.get("med", float("nan")) * 1e3, mn=p.get("min", float("nan")) * 1e3,
             tot=tot / 1e3, pre=pre / 1e3, stdform=stdform / 1e3, setup=(setup - lu0) / 1e3, lu0=lu0 / 1e3,
             main=main / 1e3, fin=fin / 1e3, post=post / 1e3, other=(other + hash_extra) / 1e3, iters=it, rounds=rounds)
    rows.append(r)
    light = tot / 1e3 < 10.0
    all_solve += tot
    if light:
        light_solve += tot
    for k in runs[-1]["pipe"]:
        if k in ("solve_mip", "presolve_total", "presolve_loop_total", "run_extended", "build_std_form_presolved", "ext_total", "ext_setup_total", "py_out"):
            continue
        v = agg(runs, k)
        step_tot[k] += v
        step_cnt[k] += runs[-1]["pipe_cnt"][k]
        if light:
            step_tot_light[k] += v

rows.sort(key=lambda r: r["med"])
print(f"{'problem':<10} {'n':>6} {'m':>6} {'n_out':>6} {'m_out':>6} {'solve':>9} {'presolve':>8} {'pre%':>5} {'stdform%':>8} {'setup%':>6} {'lu0%':>5} {'main%':>6} {'fin%':>5} {'post%':>5} {'other%':>6} {'iters':>6} {'rnd':>3}")
gm_pre = []
for r in rows:
    t = r["tot"] or 1e-9
    pct = lambda x: 100.0 * x / t
    gm_pre.append(pct(r["pre"]))
    print(f"{r['name']:<10} {r['n']:>6} {r['m']:>6} {r['n_out'] if r['n_out'] is not None else '-':>6} {r['m_out'] if r['m_out'] is not None else '-':>6} {r['med']:9.3f} {r['pre']:8.3f} {pct(r['pre']):5.1f} {pct(r['stdform']):8.1f} {pct(r['setup']):6.1f} {pct(r['lu0']):5.1f} {pct(r['main']):6.1f} {pct(r['fin']):5.1f} {pct(r['post']):5.1f} {pct(r['other']):6.1f} {r['iters']:>6} {r['rounds']:>3}")

print()
print("mean presolve share over 93 problems: %.1f%%" % (sum(gm_pre) / len(gm_pre)))
n_light = sum(1 for r in rows if r["tot"] < 10.0)
print(f"light (<10ms) problems: {n_light}, their mean presolve share: %.1f%%" % (sum(100.0 * r['pre'] / r['tot'] for r in rows if r['tot'] < 10.0) / n_light))
for lim in (1.0, 3.0, 10.0, 50.0):
    sub = [r for r in rows if r["tot"] < lim]
    if sub:
        print(f"  solve<{lim}ms: {len(sub)} problems, mean shares: presolve {statistics.mean(100*r['pre']/r['tot'] for r in sub):.1f}% stdform {statistics.mean(100*r['stdform']/r['tot'] for r in sub):.1f}% setup {statistics.mean(100*r['setup']/r['tot'] for r in sub):.1f}% lu0 {statistics.mean(100*r['lu0']/r['tot'] for r in sub):.1f}% main {statistics.mean(100*r['main']/r['tot'] for r in sub):.1f}% finish {statistics.mean(100*r['fin']/r['tot'] for r in sub):.1f}% post {statistics.mean(100*r['post']/r['tot'] for r in sub):.1f}% other {statistics.mean(100*r['other']/r['tot'] for r in sub):.1f}%")

print()
print("presolve step totals (ms) over all 93 / over light(<10ms) problems  [calls]")
for k, v in sorted(step_tot.items(), key=lambda kv: -kv[1]):
    print(f"  {k:<28} {v/1e3:9.2f}  {step_tot_light[k]/1e3:9.2f}  [{step_cnt[k]}]  light share of light solve: {100*step_tot_light[k]/light_solve:5.2f}%")
print(f"all solve total {all_solve/1e3:.1f} ms, light solve total {light_solve/1e3:.1f} ms")
json.dump(rows, open(path.replace('.json', '_table.json'), 'w'), indent=1)
