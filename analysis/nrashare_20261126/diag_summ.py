"""診断の集計: python analysis/nrashare_20261126/diag_summ.py logs_*.err
各求解 (再スタートで区切る) の終わりの、スレッドごとの閉じた葉 [実行不能, 下界] と φ・f、全スレッドの合計から計算した φ・f を出す。
NRA_TRACE の行 (500 ノードおき) から、同じ求解の中のスレッド間の f の幅 (最大 − 最小) の推移も出す。"""
import math, re, sys
A, B = 16.9, 0.49
phi = lambda i, b: (i + 1) / (i + b + 2)
f = lambda p: 1 / (1 + math.exp(-A * (p - B)))
for fn in sys.argv[1:]:
    print(f"== {fn}")
    solves, cur = [], {}
    for l in open(fn, errors="replace"):
        m = re.search(r"node rows adapt \((thread (\d+)|NODEROWS_ADAPT)\):(?: nodes (\d+))? leaves infeasible (\d+) bound (\d+)", l)
        if m:
            t = int(m.group(2)) if m.group(2) else 0
            if t in cur:  # 同じスレッドが 2 度目に出たら次の求解
                solves.append(cur); cur = {}
            cur[t] = (int(m.group(4)), int(m.group(5)))
    if cur: solves.append(cur)
    for k, s in enumerate(solves):
        if sum(i + b for i, b in s.values()) == 0: continue
        parts = " ".join(f"t{t}:{i}/{b} φ{phi(i,b):.2f} f{f(phi(i,b)):.2f}" for t, (i, b) in sorted(s.items()))
        I, Bn = sum(i for i, _ in s.values()), sum(b for _, b in s.values())
        fs = [f(phi(i, b)) for i, b in s.values()]
        print(f"  solve {k}: {parts} | pooled {I}/{Bn} φ{phi(I,Bn):.2f} f{f(phi(I,Bn)):.2f} | f range {max(fs)-min(fs):.2f}")
    # NRA_TRACE: 同じ時刻の近くのスレッド間の f の幅
    tr = [(int(m.group(1)), int(m.group(2)), float(m.group(3)), float(m.group(4))) for m in (re.search(r"NRA_TRACE tid (\d+) nodes (\d+) t ([0-9.]+) .* f ([0-9.]+) rho", l) for l in open(fn, errors="replace")) if m]
    if tr:
        rng = [max(x[3] for x in tr[i:i+4]) - min(x[3] for x in tr[i:i+4]) for i in range(0, len(tr) - 3, 4)]
        print(f"  NRA_TRACE {len(tr)} lines, f range in groups of 4 consecutive lines: mean {sum(rng)/len(rng):.2f} max {max(rng):.2f}")
