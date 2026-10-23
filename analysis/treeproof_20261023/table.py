"""設定ごとの表: python3 table.py <名前>... (logs/<問題>_<名前>_<n>.log を読む)
出す: 証明の回数、時間 (時間切れは 'TL')、60 秒の下界、ノード、最適値 (12890・6382.1) を見つけた通しの時刻、木の開始、再スタート回数"""
import glob, json, re, sys
D = '../heur_ls_20261023/logs/'
for p in ['h80x6320d', 'roll3000']:
    for name in sys.argv[1:]:
        fs = sorted(glob.glob(f'{D}{p}_{name}_*.log'), key=lambda f: int(re.search(r'_(\d+)\.log$', f).group(1)))
        rows = []
        for f in fs:
            off = 0.0; nres = 0; tree = None; inc = []
            last = None
            for l in open(f):
                m = re.match(r"Restart \d+: .*\((\d+\.\d+)s\)", l)
                if m: off += float(m.group(1)); nres += 1; continue
                m = re.search(r"new incumbent (\S+) at node (\d+) \((\d+\.\d+)s\)", l)
                if m: inc.append((float(m.group(1)), off + float(m.group(3))))
                m = re.search(r"tree search with \d+ threads \((\d+\.\d+)s\)", l)
                if m: tree = off + float(m.group(1))
                if l.startswith('{'): last = json.loads(l)
            if not last: continue
            best = last['objective']
            tf = min((t for v, t in inc if abs(v - best) <= 1e-6 * max(1, abs(best))), default=None)
            rows.append((last['status'], last['time'], last['best_bound'], last['nodes'], tf, tree, nres))
        if not rows: continue
        np_ = sum(r[0] == 'optimal' for r in rows)
        fmt = lambda r: f"{r[1]:.1f}" if r[0] == 'optimal' else f"TL({r[2]:.6g})"
        print(f"| {p} | {name} | {np_}/{len(rows)} | {' ・ '.join(fmt(r) for r in rows)} | {' ・ '.join(str(r[3]) for r in rows)} | "
              f"{' ・ '.join(f'{r[4]:.1f}' if r[4] else '-' for r in rows)} | {' ・ '.join(f'{r[5]:.1f}' if r[5] else '-' for r in rows)} | {' ・ '.join(str(r[6]) for r in rows)} |")
