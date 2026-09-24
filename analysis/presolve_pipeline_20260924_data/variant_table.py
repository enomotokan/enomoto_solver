import json, sys, math, glob, os
ref = {p['name']: p for p in json.load(open('full_ps.json'))}
def load(path):
    return {p['name']: p for p in json.load(open(path))}
base = {}
for bp in ['base3.json', 'base3b.json', 'base3c.json']:
    if os.path.exists(bp):
        for n, p in load(bp).items():
            base.setdefault(n, {'prof': []})['prof'] += p['prof']
def metric(p): return min(r['pipe']['solve_mip'] for r in p['prof'])
def sizes(p):
    c = p['prof'][-1]['pipe_cnt']; return (c.get('n_out'), c.get('m_out'), c.get('ext_iters'), c.get('rounds'))
DESC = {
 'noagg': 'aggregator 無効 (`ENOMOTO_DISABLE_AGGREGATOR`)', 'noredeq': 'reduce_equalities を丸ごとスキップ', 'noredineq_round': 'round 末尾の reduce_inequalities をスキップ',
 'nopc': 'parallelcols 無効 (`ENOMOTO_DISABLE_PARALLELCOLS`)', 'nodbl': 'doubleton 無効', 'nodprop': 'dualpropagate 無効', 'noeqprop': 'propagate_equalities (eqprop) 無効',
 'ruiz0': 'Ruiz 反復 0 (スケーリングなし)', 'ruiz3': 'Ruiz 反復 3', 'ruiz5': 'Ruiz 反復 5', 'ruiz20': 'Ruiz 反復 20', 'scalenb': 'スケーリングの走査から境界行 (単項行) を除外',
 'prop1': 'propagate の内部 pass 1', 'prop4': 'propagate の内部 pass 4', 'rounds3': '外側 round 上限 3', 'rounds5': '外側 round 上限 5',
 'reltol3': 'propagate の締め付けを相対 1e-3 以上のときだけ採用', 'reltol6': '同 1e-6', 'dense_all': '従属等式検出を常に密 QR', 'sparse_all': '同 常に疎消去',
 'minblock0': 'DM ブロック分解を行数に関係なく実施', 'norayon': 'DM 成分の rayon 並列を無効', 'base3b': '基準の再計測 (ノイズ水準)', 'base3c': '基準の再計測 2',
 'nocs': 'colsingleton 無効', 'nodualfix': 'dualfix 無効', 'nofold': 'foldfixed 無効', 'nofree': 'freevar 無効 (`ENOMOTO_DISABLE_FREEVAR`)',
 'noredineq0': 'ループ前の reduce_inequalities をスキップ', 'eqprop1': 'eqprop を round 0 のみ', 'fixexact': '固定点判定を厳密比較 (`ENOMOTO_FIXPOINT_EXACT`)',
 'rowlocal': 'row-local aggregator (`ENOMOTO_ROWLOCAL_AGGREGATOR`)', 'aggnobreak': 'aggregator の fill-in 連続失敗打ち切りなし', 'inner2': '内側 round 上限 2',
 'ineqs': 'ineqsingleton 有効 (`ENOMOTO_INEQ_SINGLETON`)', 'redeq_rank': 'reduce_equalities を重複除去のみ (rank 判定なし)', 'ineq_skipunit': 'reduce_inequalities で単項行をハッシュしない',
 'ineq_cap': 'reduce_inequalities の HashMap を容量付きで確保', 'agg_precheck': 'aggregator v2 に候補ゼロの早期 return (事前チェック)', 'combo': 'redeq_rank + ineq_skipunit + agg_precheck',
}
order = sys.argv[1:] if len(sys.argv) > 1 else sorted(glob.glob('var_*.json'))
print("| 変種 | 内容 | 幾何平均 | 合計 | 10%超退行 | サイズ/反復が変わった問題 | 目的値不一致 | 10% 以上速くなった問題 (上位) | 遅くなった問題 (下位) |")
print("|---|---|---:|---:|---:|---:|---|---|---|")
for path in order:
    name = os.path.basename(path)[4:-5]
    var = load(path)
    ratios = []; rows = []; changed = []; tb = tn = 0.0; mism = []
    for n, pv in var.items():
        if n not in base: continue
        b, v = metric(base[n]), metric(pv); tb += b; tn += v; r = v / b; ratios.append(r)
        if sizes(base[n])[:3] != sizes(pv)[:3]: changed.append(n)
        pr = ref[n]; ob, ov = pr.get('obj'), pv.get('obj')
        if pv.get('status') is not None and (pr['status'] != pv['status'] or (ob is not None and ov is not None and abs(ob - ov) > 1e-6 * max(1.0, abs(ob)))): mism.append(n)
        rows.append((n, r))
    if not ratios: continue
    gm = math.exp(sum(math.log(r) for r in ratios) / len(ratios)); rows.sort(key=lambda x: x[1])
    reg = [x for x in rows if x[1] > 1.10]
    best = ", ".join(f"{x[0]} {100*(x[1]-1):+.0f}%" for x in rows[:6] if x[1] < 0.9)
    worst = ", ".join(f"{x[0]} {100*(x[1]-1):+.0f}%" for x in rows[-6:] if x[1] > 1.05)
    print(f"| {name} | {DESC.get(name, name)} | {100*(gm-1):+.1f}% | {100*(tn/tb-1):+.1f}% ({len(ratios)} 問) | {len(reg)} | {len(changed)} | {'なし' if not mism else ', '.join(mism)} | {best} | {worst} |")
