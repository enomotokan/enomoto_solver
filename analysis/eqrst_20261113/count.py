"""fuzz のログ (@@SEED で区切る) から、再スタートが起きた問題・再スタート後に EQUIV2 の組を使った問題を数える。"""
import sys, re
seeds = {}; cur = None
for l in open(sys.argv[1], errors="replace"):
    if l.startswith("@@SEED"): cur = int(l.split()[1]); seeds[cur] = [0, 0, 0]; continue
    if cur is None: continue
    if "restart check" in l or "root LP after the restart" in l: seeds[cur][0] = 1
    if "equivalence pass with implications" in l and l.rstrip().endswith(", used"):
        seeds[cur][2] += 1
        if seeds[cur][0]: seeds[cur][1] = 1
n = len(seeds); r = sum(v[0] for v in seeds.values()); e = sum(v[1] for v in seeds.values()); u = sum(1 for v in seeds.values() if v[2])
print(f"問題 {n}、再スタートあり {r}、組を使った {u}、再スタート後に組を使った {e}")
print("再スタート後に組を使った seed:", [k for k, v in seeds.items() if v[1]][:30])
