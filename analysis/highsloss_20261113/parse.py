"""HIGHSLOSS のログ (logs/<問題>_<cand|highs>.log.gz) と結果 (res_s*.jsonl) から比べの表を作る (analysis/highsloss_20261113.md)。
python parse.py > summ.txt"""
import glob, gzip, json, re
from pathlib import Path
D = Path(__file__).resolve().parent
F = r"[-+0-9.eE]+|inf|-inf"
def ours(txt):
    d = {}
    m = re.search(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", txt)
    if m: d["size"] = tuple(map(int, m.groups()))
    m = re.search(r"^PRETIME total=([0-9.]+)", txt, re.M)
    if m: d["pre"] = float(m.group(1))
    m = re.search(r"MIP: root LP (" + F + r") \((\d+) iters, ([0-9.]+)s\)", txt)
    if m: d["rootlp"] = (float(m.group(1)), int(m.group(2)), float(m.group(3)))
    cuts = re.findall(r"cut round \d+: \d+ cuts .*? obj (" + F + r") \(([0-9.]+)s\)", txt)
    if cuts: d["cutobj"] = (float(cuts[-1][0]), float(cuts[-1][1]), len(cuts))
    tr = re.findall(r"TRSTAT: tree start outer ([0-9.]+) solve ([0-9.]+) restarts (\d+) lb (" + F + r") ub (" + F + r")", txt)
    if tr: d["tree"] = [(float(a), float(b), int(c), float(e), float(f)) for a, b, c, e, f in tr]
    prog = re.findall(r"MIP:\s+([0-9.]+)s nodes\s+(\d+) open\s+(\d+) lb (" + F + r") ub (" + F + r")", txt)
    if prog: d["prog"] = [(float(a), int(b), float(c), float(e)) for a, b, _, c, e in prog]
    d["restarts"] = len(tr) - 1 if tr else 0
    return d
def highs(txt):
    d = {}
    m = re.search(r"Solving MIP model with:\s+(\d+) rows\s+(\d+) cols \((\d+) binary, (\d+) integer, (\d+) implied int\., (\d+) continuous.*?\)\s+(\d+) nonzeros", txt, re.S)
    if m: d["size"] = (int(m.group(1)), int(m.group(2)), int(m.group(7)))
    rows = []
    for l in txt.splitlines():
        m = re.match(r"\s*([A-Za-z]?)\s+(\d+)\s+(\d+)\s+(\d+)\s+([0-9.]+)%\s+(" + F + r")\s+(" + F + r")\s+(" + F + r"|Large)%?\s+.*?\s([0-9.]+)s\s*$", l)
        if m:
            rows.append((m.group(1), int(m.group(2)), float(m.group(6)), float(m.group(7)), float(m.group(9))))
    d["rows"] = rows
    root = [r for r in rows if r[1] == 0]
    if root: d["root_end"] = (root[-1][4], root[-1][2])
    tree = [r for r in rows if r[1] > 0]
    if tree: d["tree_start"] = tree[0][4]
    sol = [r for r in rows if r[3] != float("inf")]
    if sol: d["first_sol"] = (sol[0][4], sol[0][3])
    d["restarts"] = len(re.findall(r"restarting", txt))
    return d
res = {}
for fn in glob.glob(str(D / "res_s*.jsonl")):
    for l in open(fn):
        r = json.loads(l); res[(r["problem"], r["cfg"])] = r
probs = sorted({p for p, _ in res})
for p in probs:
    print(f"== {p}")
    for c in ("cand", "highs"):
        r = res.get((p, c)); lf = D / "logs" / f"{p}_{c}.log.gz"
        if not r or not lf.exists():
            print(f"  {c}: -"); continue
        x = r["result"]; txt = gzip.open(lf, "rt").read()
        fin = f"{x.get('status')} {x.get('time', 0):.0f}s obj {x.get('objective')} bound {x.get('bound')} nodes {x.get('nodes')}"
        if c == "cand":
            d = ours(txt)
            t0 = d.get("tree", [(None,) * 5])[0]
            print(f"  cand : size {d.get('size')} pre {d.get('pre')}s rootLP {d.get('rootlp')} cuts(obj,t,rounds) {d.get('cutobj')} tree_start {t0[0]} ub_at_tree {t0[4]} restarts {d['restarts']}")
            print(f"         first_sol {x.get('first_solution_time')}s {x.get('first_solution_objective')} | {fin}")
        else:
            d = highs(txt)
            print(f"  highs: size {d.get('size')} root_end(t,bound) {d.get('root_end')} tree_start {d.get('tree_start')} restarts {d['restarts']} first_sol {d.get('first_sol')} | {fin}")
