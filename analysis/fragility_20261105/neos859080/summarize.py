"""neos859080 の各回の .out/.log から表を作る (python3 summarize.py 名前...)。"""
import json, re, sys, glob
names = sys.argv[1:] or sorted(f[:-4] for f in glob.glob("*.out"))
print("| 回 | 時間 (秒) | ノード | 根の LP 反復 | 根の下界 | カット後 | 衝突で刈り | literal 衝突で刈り | 双対証明で刈り | 分枝 (退化で強分岐なし) |")
print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---|")
for n in names:
    try:
        o = json.loads(open(n + ".out").read().strip().splitlines()[-1]); L = open(n + ".log").read()
    except Exception:
        continue
    g = lambda pat, d="-": (re.search(pat, L).group(1) if re.search(pat, L) else d)
    rootit = g(r"MIP: root LP [0-9.e-]+ \((\d+) iters")
    cutobj = g(r"cut loop done: .*-> ([0-9.e-]+)")
    lit = g(r"literal conflicts: .*pruned (\d+) nodes")
    dp = g(r"dual proofs and conflicts .*pruned (\d+) nodes")
    br = g(r"branching: (\d+ selections, \d+) without")
    t = o["time"]; st = o["status"]
    print(f"| {n} | {t:.1f}{'' if st=='infeasible' else ' ('+st+')'} | {o['nodes']:,} | {rootit} | {o['root_bound']:.4g} | {float(cutobj):.4g} | - | {lit} | {dp} | {br} |")
