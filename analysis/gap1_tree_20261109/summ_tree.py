"""ENOMOTO の木の統計 (TREE_LOG の回の stderr.gz から) を出す。使い方: python summ_tree.py <err.gz> ..."""
import gzip, re, sys, collections
for fn in sys.argv[1:]:
    L = gzip.open(fn, "rt").read().splitlines()
    enode = collections.Counter(l.split()[2] for l in L if l.startswith("ENODE"))
    depths = [int(re.search(r"d=(\d+)", l).group(1)) for l in L if l.startswith("EBR")]
    nfr = [int(m.group(1)) for l in L if l.startswith("EBR") for m in [re.search(r"nfrac=(\d+)", l)] if m]
    t4 = [l for l in L if l.startswith("TREE4")]
    last = {}
    if t4:
        t = t4[-1].split(); last = {t[k]: t[k + 1] for k in range(1, len(t) - 1)}
    br = [l for l in L if l.startswith("MIP: branching:")]
    nd = [l for l in L if l.startswith("MIP: node diag")]
    tt = [l for l in L if "tree search threads:" in l]
    dp = [l for l in L if l.startswith("MIP: dual proofs")]
    ds = sorted(depths)
    q = lambda a, f: a[int(f * (len(a) - 1))] if a else None
    print(fn.split("/")[-1])
    print("  ENODE (main+workers):", dict(enode), " branched", len(depths), " depth med/90%/max", q(ds, .5), q(ds, .9), ds[-1] if ds else None, " nfrac med", q(sorted(nfr), .5))
    print("  last TREE4: glb", last.get("glb"), "gnodes", last.get("gnodes"), "gopen", last.get("gopen"), "shared_n", last.get("shared_n"), "own_n", last.get("own_n"))
    for x in br + nd + tt + dp:
        print("  " + x[5:200])
