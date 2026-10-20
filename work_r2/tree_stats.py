"""HiGHS (HNODE/HBR) と ENOMOTO (ENODE/EBR) の木のログをまとめる。使い方: tree_stats.py ログ..."""
import re, sys, collections
HR = {0: "bound", 1: "domain", 2: "lpinf", 3: "branched", 4: "subopt", 5: "open"}
for fn in sys.argv[1:]:
    ends = collections.Counter(); bydepth = collections.defaultdict(collections.Counter)
    brcol = collections.Counter(); brdepth = collections.Counter(); rows = []
    nb = 0
    for line in open(fn, errors="replace"):
        if line.startswith("HNODE"):
            d = int(re.search(r"d=(\d+)", line).group(1)) - 1
            r = HR[int(re.search(r"r=(\d+)", line).group(1))]
            if "inbr=1" in line: r += "@sb"
            if r == "open": continue
            ends[r] += 1; bydepth[d][r] += 1
            rows.append(int(re.search(r"rows=(\d+)", line).group(1)))
        elif line.startswith("ENODE"):
            d = int(re.search(r"d=(\d+)", line).group(1)); r = re.search(r"r=(\S+)", line).group(1)
            ends[r] += 1; bydepth[d][r] += 1
            rows.append(int(re.search(r"rows=(\d+)", line).group(1)))
        elif line.startswith("HBR") or line.startswith("EBR"):
            d = int(re.search(r"d=(\d+)", line).group(1)) - (1 if line[0] == "H" else 0)
            c = int(re.search(r"col=(\d+)", line).group(1))
            brcol[c] += 1; brdepth[d] += 1; nb += 1
    print(f"== {fn}: branched {nb}, leaves {sum(ends.values())}")
    print("  ends:", dict(ends.most_common()))
    print("  distinct branched cols:", len(brcol), " top:", brcol.most_common(12))
    print("  leaves by depth:", {d: sum(bydepth[d].values()) for d in sorted(bydepth)})
    print("  branch by depth:", dict(sorted(brdepth.items())))
    if rows: print(f"  LP rows at leaves: min {min(rows)} med {sorted(rows)[len(rows)//2]} max {max(rows)}")
