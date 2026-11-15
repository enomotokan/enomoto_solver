"""ENOMOTO のログ (ENOMOTO_MIP_LOG=1) から要点を抜く: 前処理の後の大きさ、根の LP、根のカットの終わり、木の開始、暫定解の経過、終わりのノード数。
使い方: python summ_log.py ログ..."""
import re, sys
for fn in sys.argv[1:]:
    L = open(fn, errors="replace").read().splitlines()
    print("==", fn)
    for l in L:
        if re.search(r"presolve: rows|^MIP: root LP [0-9-]|symmetry: |cut loop done|background cuts: [0-9]+ rounds|tree search with|restart check|restart: |went past the cap|after root heuristics", l):
            print("  ", l[:200])
    inc = [l for l in L if "new incumbent" in l]
    for l in inc[:3] + (["   ..."] if len(inc) > 8 else []) + inc[max(3, len(inc) - 5):]:
        m = re.search(r"incumbent ([-0-9.e+]+) at node (\d+) \(([0-9.]+)s\) \[(.*)\]", l)
        print("   inc", (f"{float(m[1]):.6g} node {m[2]} {m[3]}s [{m[4]}]" if m else l[:150]))
    tl = [l for l in L if re.search(r"s nodes +\d+ open", l)]
    if tl:
        print("   last", re.sub(r"lp_iters.*", "", tl[-1]).strip()[:150])
    nd = [l for l in L if "node diag:" in l]
    if nd: print("  ", nd[-1][:200])
