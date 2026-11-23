"""根のカットのループの `separate` の下の内訳: python sepbreak.py perf.data → `separate` の直下の関数ごとのサンプル数 (ループ全体に対する %)。"""
import subprocess, sys
from collections import Counter
from dem import name
out = subprocess.run(["/usr/lib/linux-tools-6.8.0-151/perf", "script", "-i", sys.argv[1], "-F", "ip,sym", "--no-demangle"], capture_output=True, text=True).stdout
c, loop, tot = Counter(), 0, 0
for b in out.split("\n\n"):
    fr = [name(l.split(None, 1)[1].strip()) for l in b.strip().splitlines() if len(l.split(None, 1)) > 1]
    if not fr: continue
    tot += 1
    idx = [k for k, n in enumerate(fr) if "root_cut_loop_inner" in n]
    if not idx: continue
    loop += 1
    i = idx[0]
    s = [k for k in range(i) if fr[k] == "Solver::separate"]
    if not s:
        c["(separate の外) " + (fr[i - 1] if i >= 1 else "ループの関数そのもの")] += 1
        continue
    j = s[-1]
    c["separate > " + (fr[j - 1] if j >= 1 else "separate そのもの")] += 1
print(f"サンプル {tot}、根のカットのループ {loop} ({100 * loop / max(tot, 1):.1f}%)")
for k, v in c.most_common(18):
    print(f"  {100 * v / max(loop, 1):5.1f}%  {k}")
