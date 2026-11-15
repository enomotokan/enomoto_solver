"""HIGHS_GC_TRACE_MODK の出力から、LP に入った mod-k のカット (HIGHS_POOL_TAG の L 行) の generateCut の流れ (IN/BT/TR/PP/TG/UT) を取り出す。
使い方: python -I trblocks.py pooltag.txt modk_tr.txt 出力 [本数]"""
import sys
lpidx = []; last = {}; recs = {}
for line in open(sys.argv[1]):
    if line.startswith("P "):
        h = line.split("|")[0].split()
        if h[1] == "0": last[(int(h[2]), int(h[3]))] = (h[4], h[5], line)
    elif line.startswith("L "):
        f = line.split()
        if f[1] != "0": continue
        r = last.get((int(f[3]), int(f[2])))
        if r and r[0] == "modk": lpidx.append((int(f[2]), r))
want = {i for i, _ in lpidx[: int(sys.argv[4]) if len(sys.argv) > 4 else 10]}
out = open(sys.argv[3], "w"); buf = []; rnd = None; found = set()
for line in open(sys.argv[2]):
    if line.startswith("ROUND"): rnd = line.strip(); continue
    if line.startswith("IN "): buf = []
    buf.append(line if len(line) < 20000 else line[:20000] + " ...\n")
    if line.startswith("POOL "):
        k = int(line.split()[1])
        if k in want and k not in found:
            found.add(k); out.write(f"##### pool {k} ({rnd})\n"); out.writelines(buf)
print("LP-added modk cuts", len(lpidx), "found", len(found))
