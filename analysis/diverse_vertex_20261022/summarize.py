"""ログの要約: 結果・木の開始・従の根 LP・多様化の行・根の被約費用による締め付け・受け取った大域的な境界。
使い方: python summarize.py 'logs/*.log'"""
import glob, json, re, sys
for f in sorted(glob.glob(sys.argv[1] if len(sys.argv) > 1 else "logs/*.log")):
    txt = open(f).read()
    res = re.search(r"RESULT (.*)", txt)
    r = json.loads(res.group(1)) if res else {}
    tree = re.search(r"tree search with 4 threads \(([\d.]+)s\)", txt)
    root = re.findall(r"tid (\d) root LP (\w+) from the snapshot basis (\w+) \(dse \w+\): (\d+) iters, ([\d.]+)s .*?(?:.*vs the snapshot LP point: (\d+) int cols differ.*?reduced costs differ in (\d+))?", txt, re.M)
    dv = re.findall(r"tid (\d) diverse vertex \w+.*?d-solve (\w+) (\d+) iters.*?c-solve (\w+) (\d+) iters, ([\d.]+)s, (kept|fell back to x0).*?final (\d+) \(L1", txt)
    dd = re.findall(r"tid (\d) diverse dual: (\d+) candidates, try (\d) .*?perturbed solve (\w+) .*?total (\d+) iters, ([\d.]+)s, (kept|fell back)", txt)
    dg = re.findall(r"tid (\d) diverse DvMode.*?duals differ in (\d+) of \d+ rows, int reduced costs differ in (\d+) \(nonzero (\d+) -> (\d+)\)", txt)
    ends = re.findall(r"tid (\d) end: root reduced-cost tightenings (\d+) \(candidates (\d+)\), global bounds taken from other threads (\d+), nodes (\d+)", txt)
    print(f"{f.split('/')[-1]:40} {str(r.get('status')):10} obj {r.get('objective')} bnd {r.get('bound')} t {r.get('time')} nodes {r.get('nodes')} tree@{tree.group(1) if tree else '-'}")
    if root:
        print("   root LP (tid,iters,s,int-diff,rc-diff):", [(a[0], a[3], a[4], a[5], a[6]) for a in root])
    if dv:
        print("   primal walk (tid,d-it,c-it,s,ok,moved):", [(a[0], a[2], a[4], a[5], a[6][:4], a[7]) for a in dv])
    if dd:
        print("   dual (tid,cand,try,st,it,s,ok):", [(a[0], a[1], a[2], a[3][:3], a[4], a[5], a[6][:4]) for a in dd])
    if dg:
        print("   dual diff (tid,rows,rc,nz0->nz):", [(a[0], a[1], a[2], f"{a[3]}->{a[4]}") for a in dg])
    if ends:
        print("   end (tid,rcfix,cand,pulled,nodes):", sorted([(a[0], a[1], a[2], a[3], a[4]) for a in ends]))
