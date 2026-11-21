"""diag.sh のログの要約: 時間・ノード・木の 1 ノードあたりの LP の反復と時間・根の終わりと木の終わりの LP の行・根のカットの種類ごとの本数。
使い方: python diag_summ.py logs/diag_*_t1_s1.log"""
import json, re, sys
for f in sys.argv[1:]:
    L = open(f).read().splitlines()
    res = json.loads([l for l in L if "@@RESULT" in l][-1].split("@@RESULT", 1)[1])
    ts = [l for l in L if "TRSTAT: tree start" in l]
    t_tree0 = float(re.search(r"outer ([0-9.]+)", ts[-1]).group(1)) if ts else res["time"]
    prog = [l for l in L if re.search(r"MIP: +[0-9.]+s nodes +\d+", l)]
    ni = lps = None; rows_end = None
    if prog:
        m = re.search(r"node_iters (\d+) \(first \d+, lps (\d+)\)", prog[-1]); ni, lps = (int(m.group(1)), int(m.group(2))) if m else (None, None)
        m = re.search(r"lp_rows (\d+)", prog[-1]); rows_end = int(m.group(1)) if m else None
    cl = [l for l in L if "cut loop done" in l]
    root = re.search(r"(\d+) in LP, pool (\d+) cuts \((\d+) nonzeros\), obj ([-0-9.e+]+) -> ([-0-9.e+]+)", cl[0]) if cl else None
    rr = [l for l in L if re.search(r"cut round \d+: \d+ cuts \(of", l)]
    last_rows = re.search(r"LP rows (\d+) \(cut rows mean nnz (\d+)\)", rr[0 if not cl else len([l for l in rr])-1]) if rr else None
    kinds = {}
    for l in L:
        m = re.search(r"SEPLEARN root restarts 0 root (\w+) +calls \d+ cands (\d+) chosen (\d+)", l)
        if m and m.group(3) != "0": kinds[m.group(1)] = (int(m.group(2)), int(m.group(3)))
    tree_t = res["time"] - t_tree0
    n = res["nodes"]
    print(f"{f.split('/')[-1][5:-4]:22s} {res['time']:6.1f}s nodes {n:6d} root_end {t_tree0:5.1f}s",
          f"tree {tree_t:5.1f}s" + (f" ({1000*tree_t/max(n,1):.1f} ms/node, {ni/max(lps,1):.1f} it/LP, {ni/max(n,1):.1f} it/node)" if ni else ""),
          f"| root: {root.group(1)} cuts in LP, obj {float(root.group(5)):.6g}, pool nnz {root.group(3)}" if root else "",
          f"| LP rows end {rows_end}" if rows_end else "",
          "| " + " ".join(f"{k} {c}/{h}" for k, (h, c) in kinds.items()))
