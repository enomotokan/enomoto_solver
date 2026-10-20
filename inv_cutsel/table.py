"""logs/<問題>_<設定>_<回>.log から問題ごとの表 (Markdown) を作る。"""
import re, json, sys, glob, collections
def parse(fn):
    roots, rst, tree_t, res = [], 0, None, None
    for l in open(fn):
        if 'Root cuts:' in l:
            roots.append(float(re.search(r'-> ([-\d.e+]+)', l).group(1)))
        elif re.search(r'Restart \d+: ', l):
            rst += 1
        elif tree_t is None and re.match(r'\[\s*[\d.]+\]\s+[\d.]+s\s+\d+', l):
            tree_t = float(re.search(r'\[\s*([\d.]+)\]', l).group(1))
        elif 'RESULT ' in l:
            res = json.loads(l.split('RESULT ', 1)[1])
    return roots, rst, tree_t, res
probs = ["neos-911970", "roll3000", "mzzv42z", "neos-2978193-inde"]
cfgs = sys.argv[1:]
for p in probs:
    print(f'\n### {p}\n\n| 設定 | 回 | 根の下界 (各回の根) | 再スタート | 木に入る [s] | 結果 | 上界 | 下界 | ノード |\n|---|---|---|---|---|---|---|---|---|')
    for c in cfgs:
        for fn in sorted(glob.glob(f'logs/{p}_{c}_[ab].log')):
            roots, rst, tt, r = parse(fn)
            if r is None: continue
            st = f"**{r['time']:.1f} 秒**" if r['status'] == 'optimal' else '時間切れ'
            ub = r['objective']; ub = f'{ub:.6g}' if ub is not None else '-'
            print(f"| {c} | {fn[-5]} | {' / '.join(f'{x:.6g}' for x in roots)} | {rst} | {tt if tt is not None else '-'} | {st} | {ub} | {r['bound']:.6g} | {r['nodes']} |")
