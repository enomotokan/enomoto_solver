"""ログの要約: 結果、根の下界 (前景・背景のカットの後)、根のヒューリスティクスの後の暫定解、暫定解の推移、
大域の下界 (20/30/40 秒)、主スレッドのノード。使い方: python summ.py <log>..."""
import re, sys
for fn in sys.argv[1:]:
    L = open(fn).read().splitlines()
    res = next((l[7:] for l in L if l.startswith('RESULT')), '?')
    fg = [l for l in L if l.startswith('MIP: cut round')]
    fgobj = re.search(r'obj ([\d.e+]+)', fg[-1]).group(1) if fg else '?'
    bg = next((re.search(r'-> ([\d.e+]+)', l).group(1) for l in L if 'background cuts:' in l and 'rounds' in l and '->' in l), '?')
    ah = next((re.search(r'incumbent (\S+)', l).group(1) for l in L if 'after root heuristics' in l), '?')
    inc = [(re.search(r'incumbent ([\d.e+]+) at node (\d+) \(([\d.]+)s\) \[(.*)\]', l)) for l in L if 'new incumbent' in l]
    inc = [f"{float(m.group(1)):.0f}@{float(m.group(3)):.1f}({m.group(4).replace('other thread: ','o:')})" for m in inc if m]
    tab = [l.split() for l in L if re.match(r'^\s+[\d.]+s\s+\d+\s+\d+\s', l)]
    def at(t):
        r = [x for x in tab if float(x[0][:-1]) <= t]
        try: return f"{float(r[-1][3]):.1f}" if r else '-'
        except ValueError: return r[-1][3]
    diag = [l for l in L if l.startswith('MIP: node diag')]
    mn = re.search(r'nodes (\d+)', diag[-1]).group(1) if diag else '?'
    print(fn.split('/')[-1], res)
    print(f"  root fg {float(fgobj):.2f} bg {bg} after-heur {ah} | glob lb@20 {at(20)} @30 {at(30)} @40 {at(40)} @60 {at(61)} | main nodes {mn}")
    print('  inc:', ' '.join(inc[-12:]))
