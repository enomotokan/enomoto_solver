"""beasleyC3 の全回の表 (背景のカットの後の根の下界の順)。使い方: python table.py"""
import re, glob, json
from pathlib import Path
D = Path(__file__).resolve().parent / 'logs'
rows = []
for f in sorted(D.glob('*_[0-9].log')):
    n = f.stem
    if n.startswith(('highs', 'symt1_0', 'symt3', 'symt10')) or '_' in n.rsplit('_', 1)[0] and not n.startswith(('p5s', 'def', 'symt', 'nosym', 'ext', 'reh', 'rens', 'rdive', 'defco', 'symt1co')):
        continue
    if not re.match(r'^(def|defb|fcsub|p5sfcsub|p5s|symt1|nosym|ext|ext2|scr|scr2|scrdv|sc|scdive|screh|r516|p5sext|reh|rens|rdive|defco|symt1co|exthv|extipm|p5sextipm)_\d$', n):
        continue
    L = f.read_text()
    r = re.search(r'RESULT (.*)', L)
    if not r:
        continue
    r = json.loads(r.group(1))
    fg = re.findall(r'cut round \d+: .*obj ([\d.e+]+)', L)
    bg = re.search(r'background cuts: \d+ rounds.*-> ([\d.e+]+)', L)
    sym = re.search(r'symmetry: (\d+) gen', L)
    race = re.findall(r'Root LP: the ([a-z-]+)', L)
    t754 = re.findall(r'new incumbent 7\.54\d+e2 at node \d+ \(([\d.]+)s\)', L)
    t756 = re.findall(r'new incumbent 7\.5[0-6]\d+e2 at node \d+ \(([\d.]+)s\)', L)
    rows.append((float(bg.group(1)) if bg else 0, n, sym.group(1) if sym else '-', 'IPM' if race and race[-1].startswith('interior') else '2相', f"{float(fg[-1]):.2f}" if fg else '-',
                 '解' if r['status'] == 'optimal' else f"TL {r['bound']:.1f}/{r['obj']:.0f}", f"{r['time']:.1f}", t756[0] if t756 else '—', t754[0] if t754 else '—', r['nodes']))
print('| 回 | 対称性の生成元 | 根の LP | 前景のカットの後 | 背景のカットの後 | 結果 | 時間 | ≤756 の時刻 | 754 の時刻 | ノード |')
print('|---|---|---|---|---|---|---|---|---|---|')
for x in sorted(rows):
    print(f"| {x[1]} | {x[2]} | {x[3]} | {x[4]} | {x[0]:.2f} | {x[5]} | {x[6]} | {x[7]} | {x[8]} | {x[9]} |")
