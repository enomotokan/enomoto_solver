"""heaptrack_print の出力 (PEAK MEMORY CONSUMERS の節) を、確保の場所ごとに src の枠だけに縮め、LU の写し・FT の更新・gemm の
作業領域・そのほかに分けて合計する (2026-11-03)。使い方: heaptrack_print -f ht.gz > ht.txt; python ht_summary.py ht.txt [上位の数]"""
import re, sys
L = open(sys.argv[1], errors="replace").read().splitlines()
top = int(sys.argv[2]) if len(sys.argv) > 2 else 10
s = next(i for i, l in enumerate(L) if l.startswith("PEAK MEMORY CONSUMERS"))
e = next(i for i, l in enumerate(L) if l.startswith("MOST TEMPORARY ALLOCATIONS"))
P = L[s:e]
def mb(x):
    m = re.match(r"([0-9.]+)([KMG]?)", x); return float(m.group(1)) * {"": 1e-6, "K": 1e-3, "M": 1, "G": 1e3}[m.group(2)]
blocks = []
for i, l in enumerate(P):
    m = re.match(r"^([0-9.]+[KMG]?) peak memory consumed over (\d+) calls", l)
    if m:
        blocks.append([mb(m.group(1)), int(m.group(2)), []]); continue
    if blocks:
        f = re.search(r"at (?:/home/user/enomoto_solver/)?(src/\S+)", l)
        if f and "/rustc/" not in l and ".cargo" not in l:
            name = P[i - 1].strip().split("(")[0][:60]
            fr = f"{name} @{f.group(1)}"
            if fr not in blocks[-1][2]: blocks[-1][2].append(fr)
def kind(b):
    t = "\n".join(b[2])
    if "mip_lp.rs:1219" in t: return "LU の写し (mip_lp.rs:1219、共有中の FtLu の複製)"
    if "commit_update_tracked" in t: return "FT の更新の eta (lu.rs commit_update_tracked)"
    if "multifrontal" in t: return "内点法 (multifrontal、gemm の作業領域を含む)"
    return "そのほか"
tot = {}
for b in blocks:
    tot[kind(b)] = tot.get(kind(b), 0) + b[0]
for l in L[-8:]:
    if l.strip(): print(l)
print("\n分類ごとの合計 [MB] (最大のときに生きていた確保):")
for k, v in sorted(tot.items(), key=lambda x: -x[1]): print(f"  {v:9.1f}  {k}")
print(f"\n上位 {top} の確保の場所 (src の枠だけ、最初の 12 枠):")
for b in blocks[:top]:
    print(f"- {b[0]:.1f} MB ({b[1]} 回) {kind(b)}")
    for fr in b[2][:12]: print("    " + fr)
