"""分枝したノードの LP 値の深さごとの平均 (HiGHS の HBR は LP 値を出さないので HNODE r=5 (open) を使う)。"""
import re, sys, collections
for fn in sys.argv[1:]:
    by = collections.defaultdict(list)
    for line in open(fn, errors="replace"):
        if line.startswith("HNODE") and " r=5 " in line:
            d = int(re.search(r"d=(\d+)", line).group(1)) - 1
            by[d].append(float(re.search(r"lp=(\S+)", line).group(1)))
        elif line.startswith("EBR"):
            d = int(re.search(r"d=(\d+)", line).group(1))
            by[d].append(float(re.search(r"lp=(\S+)", line).group(1)))
    s = " ".join(f"{d}:{sum(v)/len(v):.4f}({len(v)})" for d, v in sorted(by.items()) if d <= 24)
    print(fn, s)
