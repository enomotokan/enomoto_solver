"""計測の比較: [BENCH_SHA=<commit 7 桁>] python scripts/bench_cmp_heur_20261026.py 名前 ... (benchmarks/*_<commit>_<名前>[_g<組>]_r<回>.json を集める)。"""
import json, math, sys, glob, re
from pathlib import Path
REPO = Path('/home/user/enomoto_solver')
SOLU = {}
for l in open(Path(__file__).parent / 'miplib2017-v31.solu'):
    t = l.split()
    if len(t) >= 3 and t[0] in ('=opt=', '=best='):
        SOLU[t[1]] = float(t[2])
import os
# 計測したコードの commit (ファイル名の 7 桁)。`BENCH_SHA` で変える
SHA = os.environ.get('BENCH_SHA', '0d48899')

def load(name):
    runs = {}
    for suite in ('miplib_results', 'highsonly16'):
        # 名前の後に組の接尾辞 (_g1 など) があってもよい (scripts/bench_interleave.sh)
        pats = [f'benchmarks/{suite}_4t_60s_20261020_{SHA}_{name}_r*.json', f'benchmarks/{suite}_4t_60s_20261020_{SHA}_{name}_g*_r*.json']
        for f in sorted(x for pat in pats for x in glob.glob(str(REPO / pat))):
            r = int(re.search(r'_r(\d+)\.json$', f).group(1))
            stem = re.search(rf'_{SHA}_(.+)_r\d+\.json$', f).group(1)
            log = REPO / f'benchmarks/cand_logs/{stem}_r{r}.{"40" if suite == "miplib_results" else "16"}.log'
            done = {l.split()[0] for l in open(log)} if log.exists() else set()
            d = json.load(open(f))
            for p, v in d.items():
                if p in done:
                    runs.setdefault(r, {})[(suite, p)] = v['ours']
    return runs

def pgap(o, p):
    ref = SOLU.get(p)
    obj = o.get('objective')
    if obj is None or ref is None:
        return 1.0
    if abs(obj - ref) <= 1e-6 * max(1, abs(ref)):
        return 0.0
    if obj * ref < 0:
        return 1.0
    return abs(obj - ref) / max(abs(obj), abs(ref))

def tm(o):
    return o['time'] if o.get('status') == 'optimal' and o.get('time') is not None else 60.0

names = sys.argv[1:]
data = {n: load(n) for n in names}
# 全部の設定・回で測った問題だけを比べる
common = None
for n in names:
    for r, d in data[n].items():
        common = set(d) if common is None else common & set(d)
common = sorted(common or [])
print(f'problems in common: {len(common)}')
print(f'{"config":10} {"run":>3} {"opt":>4} {"SGM10":>7} {"pgap_sum":>8} {"wrong":>5}')
per = {}
for n in names:
    for r in sorted(data[n]):
        d = data[n][r]
        opt = sum(d[k].get('status') == 'optimal' for k in common)
        sgm = math.exp(sum(math.log(tm(d[k]) + 10) for k in common) / len(common)) - 10
        pg = sum(pgap(d[k], k[1]) for k in common)
        wrong = [k[1] for k in common if d[k].get('status') == 'optimal' and pgap(d[k], k[1]) > 1e-6]
        print(f'{n:10} {r:>3} {opt:>4} {sgm:7.2f} {pg:8.3f} {len(wrong):>5} {" ".join(wrong)}')
        for k in common:
            per.setdefault(k, {}).setdefault(n, []).append(d[k])
# 問題ごとの違い (状態か目的値が設定で違うもの)
print()
print('per problem (status/time or objective per run):')
for k in common:
    row = per[k]
    def cell(o):
        return f'{o["time"]:.1f}' if o.get('status') == 'optimal' else f'[{o.get("objective")}]'
    cells = {n: ' '.join(cell(o) for o in row[n]) for n in names}
    if len(set(cells.values())) > 1 and not all(all(o.get('status') == 'optimal' for o in row[n]) for n in names) or any(
            abs(math.log(tm(o) + 10) - math.log(tm(row[names[0]][0]) + 10)) > 0.3 for n in names for o in row[n]):
        print(f'  {k[1]:22} ' + ' | '.join(f'{n}: {cells[n]}' for n in names))
