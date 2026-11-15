"""mitt600 の結果から「ENOMOTO が時間切れで、下界が根の値からほとんど動いていない」問題を抜き出す。
frac = (bound - root_bound) / (objective - root_bound)。frac <= 0.01 かつ nodes >= 1000 を「動いていない」とする。
HiGHS の下界の進み hfrac も出す (HiGHS は目的値が整数なら終わりの報告で下界を切り上げるので、hb = ceil(rb) は切り上げだけの見込み)。"""
import json, glob, math, sys
rows = {}
for f in glob.glob(sys.argv[1] + '/res_s*.jsonl'):
    for l in open(f):
        try:
            r = json.loads(l)
        except Exception:
            continue
        rows.setdefault(r['problem'], {})[r['solver']] = r
tl = 0; flat = []
for p, d in sorted(rows.items()):
    o = d.get('ours'); h = d.get('highs')
    if not o or o['result'].get('status') != 'time_limit':
        continue
    tl += 1
    R = o['result']; rb, b, z = R.get('root_bound'), R.get('bound'), R.get('objective')
    if rb is None or b is None or z is None or abs(z - rb) < 1e-9:
        continue
    frac = (b - rb) / abs(z - rb)
    hb = h['result'].get('bound') if h else None
    hs = h['result'].get('status') if h else None
    hfrac = (hb - rb) / abs(z - rb) if hb is not None else None
    ceil_only = hb is not None and abs(hb - math.ceil(rb - 1e-6)) < 1e-6 and hb - rb < 1
    if frac <= 0.01 and R.get('nodes', 0) >= 1000:
        flat.append((p, rb, b, z, R['nodes'], hb, hs, hfrac, ceil_only))
print(f"ENOMOTO 時間切れ (暫定解あり・root_bound あり含む全体) {tl} 問、うち下界が動かない (frac<=1%、ノード>=1000) {len(flat)} 問")
print("| 問題 | 根の下界 | 下界 | 暫定解 | ノード | HiGHS 下界 | HiGHS 状態 | HiGHS の進み | HiGHS は切り上げだけ |")
print("|---|---:|---:|---:|---:|---:|---|---:|---|")
for x in flat:
    print(f"| {x[0]} | {x[1]:.6g} | {x[2]:.6g} | {x[3]:.6g} | {x[4]} | {x[5] if x[5] is None else f'{x[5]:.6g}'} | {x[6]} | {'' if x[7] is None else f'{x[7]:.3f}'} | {'yes' if x[8] else ''} |")
