"""ログから手間と時間を拾う (調整値の較正用)。使い方: python3 calib.py <ラベル>  (work/logs/<問題>_<ラベル>.log)
出力: 問題ごとに プロービング (回ごとの 試した/候補・秒・CPU 秒・手間)、LP による固定 (最初の LP の秒・反復・大きさ、合計の反復・秒)、
対称性 (秒・手間)。"""
import sys, re, glob, os
lab = sys.argv[1]
for f in sorted(glob.glob(f'work/logs/*_{lab}.log')):
    p = os.path.basename(f)[:-len(lab) - 5]
    t = open(f, errors='replace').read()
    pr = re.findall(r'probing: (\d+) of (\d+) binaries probed \((\d+) passes\), (\d+) fixed, (\d+) bounds tightened, \d+ implications \(([\d.]+)s, cpu ([\d.]+)s, work (\d+)(?:, pwork (\d+))?\)', t)
    lp = re.findall(r'LP probing: root LP ([\d.]+)s \((\d+) iters, size (\d+)\), tried (\d+), fixed (\d+), basis passed (\w+), total (\d+) iters ([\d.]+)s', t)
    st = re.findall(r'LP probing: the first LP stopped: (\w+) \(([\d.]+)s', t)
    sy = re.findall(r'symmetry: (.*?)\(([\d.]+)s, work (\d+)(?:, pwork (\d+))?\)', t)
    pt = re.findall(r'presolve finished in ([\d.]+)s', t)
    out = [p, 'pt=' + (pt[0] if pt else '?')]
    for a in pr:
        out.append(f'PR {a[0]}/{a[1]} p{a[2]} f{a[3]} t{a[4]} {a[5]}s cpu{a[6]} w{int(a[7]):.3g} pw{(int(a[8]) if a[8] else 0):.3g}')
    for a in lp:
        out.append(f'LP root {a[0]}s {a[1]}it sz{a[2]} rw{int(a[1])*int(a[2]):.3g} tried{a[3]} fix{a[4]} basis={a[5]} tot{a[6]}it {a[7]}s tw{int(a[6])*int(a[2]):.3g}')
    for a in st:
        out.append(f'LP first stopped {a[0]} {a[1]}s')
    for a in sy:
        out.append(f'SYM {a[1]}s w{int(a[2]):.3g} {a[0].strip()[:40]}')
    print(' | '.join(out))
