"""sumh.py <名前>...: logs/<名前>.{log,tr} から探りの数・前処理後の大きさ・根のラウンドの下界 (元の空間、最適 49324 から定数を逆算)・時間"""
import sys, re
for nm in sys.argv[1:]:
    log = open(f'logs/{nm}.log').read(); tr = open(f'logs/{nm}.tr').read().splitlines()
    pe = re.findall(r'(\d+) probing evaluations: (\d+) deleted rows, (\d+) deleted columns, (\d+) lifted', log)
    pr = re.search(r'Presolve reductions: rows (\d+)\(.*?columns (\d+)\(.*?nonzeros (\d+)', log)
    st = re.search(r'Status\s+(\S+)', log).group(1); pb = re.search(r'Primal bound\s+(\S+)', log).group(1); tm = re.search(r'Timing\s+(\S+)', log).group(1)
    inc = [l for l in tr if l.startswith('TR INCUMBENT lvl=0')]
    # 定数: 最初の暫定解の計測の値と、node log の最初の BestSol (元の空間) の差
    bs = re.search(r'^\s*\S?\s+\d+\s+\d+\s+\d+\s+\S+%\s+\S+\s+([0-9.e+-]+)\s', log, re.M)
    off = float(bs.group(1)) - float(re.search(r'obj=(\S+)', inc[0]).group(1)) if inc and bs else None
    rounds = []
    for l in tr:
        if l.startswith('TR CUTROUND lvl=0'):
            t = float(re.search(r't=(\S+)', l).group(1)); v = float(re.search(r'lpobj=(\S+)', l).group(1))
            rounds.append((t, v + off if off is not None else v))
        if l.startswith('TR INCUMBENT lvl=0') or 'RESTART' in l: break
    first = next((t for t, v in rounds if v > 49323.9), None)
    print(f"{nm}: probe {[(int(a),int(c),int(d)) for a,b,c,d in pe]}  pre {pr.groups() if pr else None}  {st} {pb} {tm}s  offset {off}")
    print("   root:", ' '.join(f'{v:.0f}@{t:.1f}' for t, v in rounds[:14]), f" 49324 at {first}")
