# SEPPROF (根のループの最初、restarts 0) の表: python psum.py <接尾辞>...  (logs/<問題>_t1_<接尾辞>.err)
import re, sys, os
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'logs')
PS = ['roll3000', 'mzzv42z', '30n20b8', 'neos-911970', 'physiciansched6-2']
for p in PS:
    for suf in sys.argv[1:]:
        f = f"{D}/{p}_t1_{suf}.err"
        if not os.path.exists(f):
            continue
        L = [l for l in open(f) if l.startswith('SEPPROF restarts 0')]
        if not L:
            continue
        h = L[0]
        g = lambda key, s: float(re.search(re.escape(key) + r" ([\d.]+)", s).group(1))
        cur = L[1]; face = L[2]
        nr = int(re.search(r"rounds (\d+)", h).group(1))
        names = ['rows', 'zerohalf', 'implbd', 'path_highs', 'path', 'tab_btran_agg', 'tab_gen']
        sc = g('sep@cur', cur); sf = g('sep@face', face)
        parts = ' '.join(f"{k} {g(k, cur) + g(k, face):.1f}" for k in names)
        print(f"| {p} | {suf} | {nr} | {g('round', h):.0f} | {sc + sf + g('face LP', h) + g('face prep', h):.1f} | {g('face LP', h):.1f} | {sc:.1f} | {sf:.1f} | {parts} | {float(re.search(r"LP ([\d.]+) remove", h).group(1)):.1f} |")
