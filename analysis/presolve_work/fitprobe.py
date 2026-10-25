"""プロービングの CPU 時間を手間の内訳 (走査した行の長さ w・更新した活動 act・境界の変更 chg・含意 imp・試した数 pr) で回帰する (較正用)。
使い方: python3 fitprobe.py <ラベル>"""
import re, glob, os, sys, numpy as np
rows = []
for f in glob.glob(f'work/logs/*_{sys.argv[1]}.log'):
    t = open(f, errors='replace').read()
    for m in re.finditer(r'probing: (\d+) of (\d+) binaries probed \((\d+) passes\), (\d+) fixed, (\d+) bounds tightened, (\d+) implications \(([\d.]+)s, cpu ([\d.]+)s, work (\d+), pwork (\d+), chg (\d+)\)', t):
        pr, _, _, _, _, imp, _, cpu, w, pw, chg = m.groups()
        pr, w, pw, imp, chg, cpu = int(pr), int(w), int(pw), int(imp), int(chg), float(cpu)
        if cpu < 0.15:
            continue
        rows.append((os.path.basename(f), cpu, w, pw - w - 100 * pr, chg, imp, pr))
cols = [2, 3, 4, 5, 6]
A = np.array([[r[c] for c in cols] for r in rows], float); b = np.array([r[1] for r in rows])
x, *_ = np.linalg.lstsq(A / b[:, None], np.ones(len(b)), rcond=None)
print('coef (w, act, chg, imp, pr)', x)
for r, p in zip(rows, A @ x):
    print(f"{r[0]:32s} cpu {r[1]:.2f} pred {p:.2f}  w{r[2]:.2g} act{r[3]:.2g} chg{r[4]:.2g} imp{r[5]:.2g} pr{r[6]}")
