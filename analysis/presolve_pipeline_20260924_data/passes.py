import json, sys
from collections import defaultdict
d = json.load(open(sys.argv[1]))
PASSES = [("reduce_equalities", "red_eq_dropped"), ("reduce_inequalities", "red_ineq0_dropped"), ("reduce_inequalities(round)", "red_ineq_round_dropped"),
          ("dualfix", "dualfix_fixes"), ("dualpropagate", None), ("rowsingleton", "rs_fixes"), ("doubleton", "dbl_subs"), ("colsingleton", "cs_subs"),
          ("aggregator", "agg_subs"), ("parallelcols", "pc_subs"), ("freevar", "free_subs"), ("eqprop", None), ("propagate", None),
          ("scaling::compute", None), ("scaling::apply", None), ("foldfixed(A)", None), ("foldfixed(G)", None), ("rebuild_g(inner)", None), ("extract_bounds(inner)", None), ("final_propagate", None)]
tot_time = defaultdict(float); tot_light = defaultdict(float); tot_red = defaultdict(int); calls = defaultdict(int); empty = defaultdict(int); nprob_hit = defaultdict(int)
light_solve = all_solve = 0.0
per = []
for p in d:
    runs = p['prof']; last = runs[-1]; cnt = last['pipe_cnt']
    t = {k: min(r['pipe'].get(k, 0.0) for r in runs) for k in last['pipe']}
    solve = t['solve_mip']; light = solve < 10000
    all_solve += solve; light_solve += solve if light else 0
    row = {'name': p['name'], 'solve': solve, 'rounds': cnt.get('rounds', 0), 'n_in': cnt.get('n_in'), 'n_out': cnt.get('n_out'), 'm_in': cnt.get('m_in'), 'm_out': cnt.get('m_out'), 'iters': cnt.get('ext_iters')}
    for name, ck in PASSES:
        tt = t.get(name, 0.0); tot_time[name] += tt
        if light: tot_light[name] += tt
        calls[name] += cnt.get(name, 0)
        red = cnt.get(ck, 0) if ck else (cnt.get('dprop_eqs', 0) + cnt.get('dprop_fixes', 0) if name == 'dualpropagate' else 0)
        tot_red[name] += red
        if red > 0: nprob_hit[name] += 1
        row[name] = (tt, cnt.get(name, 0), red)
    row['agg_empty'] = cnt.get('agg_empty_calls', 0); row['pc_empty'] = cnt.get('pc_empty_calls', 0)
    row['red_eq_path'] = 'dense' if cnt.get('red_eq_dense') else 'sparse'; row['red_eq_dm'] = cnt.get('red_eq_dm_components', 0); row['red_eq_rayon'] = cnt.get('red_eq_rayon', 0); row['red_eq_dedup'] = cnt.get('red_eq_dedup_dropped', 0)
    per.append(row)
print(f"{'pass':<28} {'time_all_ms':>11} {'time_light_ms':>13} {'%light_solve':>12} {'calls':>6} {'reductions':>10} {'#prob_hit':>9}")
for name, _ in PASSES:
    print(f"{name:<28} {tot_time[name]/1e3:11.2f} {tot_light[name]/1e3:13.2f} {100*tot_light[name]/light_solve:12.2f} {calls[name]:6d} {tot_red[name]:10d} {nprob_hit[name]:9d}")
print(f"light solve total {light_solve/1e3:.1f}ms all {all_solve/1e3:.1f}ms")
print()
print("aggregator: empty calls total", sum(r['agg_empty'] for r in per), "of", calls['aggregator'], "; parallelcols empty", sum(r['pc_empty'] for r in per), "of", calls['parallelcols'])
print("reduce_equalities: problems where it dropped rows:", [(r['name'], r['reduce_equalities'][2], r['red_eq_path']) for r in per if r['reduce_equalities'][2] > 0])
print("reduce_equalities dedup drops:", [(r['name'], r['red_eq_dedup']) for r in per if r['red_eq_dedup'] > 0])
print("reduce_equalities rayon used:", [r['name'] for r in per if r['red_eq_rayon']])
print("reduce_eq DM components>1:", [(r['name'], r['red_eq_dm']) for r in per if r['red_eq_dm'] > 1])
print("reduce_inequalities(round) drops:", [(r['name'], r['reduce_inequalities(round)'][2]) for r in per if r['reduce_inequalities(round)'][2] > 0])
print("reduce_inequalities(pre) drops:", [(r['name'], r['reduce_inequalities'][2]) for r in per if r['reduce_inequalities'][2] > 0])
print("dualfix fixes:", [(r['name'], r['dualfix'][2]) for r in per if r['dualfix'][2] > 0])
print("rowsingleton fixes:", [(r['name'], r['rowsingleton'][2]) for r in per if r['rowsingleton'][2] > 0])
print("freevar subs:", [(r['name'], r['freevar'][2]) for r in per if r['freevar'][2] > 0])
print("parallelcols subs:", [(r['name'], r['parallelcols'][2]) for r in per if r['parallelcols'][2] > 0])
print("dualpropagate reds:", [(r['name'], r['dualpropagate'][2]) for r in per if r['dualpropagate'][2] > 0])
print("rounds distribution:", sorted(((r['rounds'], r['name']) for r in per), reverse=True)[:20])
json.dump(per, open(sys.argv[1].replace('.json', '_passes.json'), 'w'), indent=1)
