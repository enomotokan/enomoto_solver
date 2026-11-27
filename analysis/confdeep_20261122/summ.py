"""diag_<名前>.jsonl の CONFDEEP の行を表にする (README 2 節)。python summ.py 名前"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / f"diag_{sys.argv[1]}.jsonl")]
F = ["made", "used", "fix", "prune", "aged", "aused", "over", "oused", "life", "first"]
def parse(line):
    out = {}
    for m in re.finditer(r"\[(\d)\] made (\d+) used (\d+) fix (\d+) prune (\d+) aged (\d+) \(used (\d+)\) overwritten (\d+) \(used (\d+)\) life (\d+) first-use (\d+)", line):
        out[int(m.group(1))] = dict(zip(F, map(int, m.groups()[1:])))
    return out
for r in rows:
    L = r["lines"]; res = r["result"]
    nodes = res.get("nodes") or 1
    pool = L.get("MIP: CONFDEEP pool", "") or next((v for k, v in L.items() if k.startswith("CONFDEEP pool")), "")
    m = re.search(r"propagate calls (\d+), examined (\d+), mean live ([\d.]+), ([\d.]+)s; age at use \(.*?\) \[([^\]]*)\]; relearned after aging (\d+) \(of used (\d+)\)", pool)
    print(f"== {r['problem']}: {res.get('status')} {res.get('time', 0):.1f}s nodes {nodes}")
    if m:
        calls, ex, live, secs = int(m.group(1)), int(m.group(2)), float(m.group(3)), float(m.group(4))
        ages = list(map(int, m.group(5).split(",")))
        tot = sum(ages) or 1
        print(f"  pool: live {live:.0f}, examined/node {ex / nodes:.0f}, prop {secs:.2f}s ({secs / max(res.get('time', 1), 1e-9) * 100:.1f}% of time, {secs / nodes * 1e6:.0f}us/node); uses at age>=64 {sum(ages[7:]) / tot:.3f}, >=256 {sum(ages[9:]) / tot:.3f}; relearned {m.group(6)} (used {m.group(7)})")
    for k, v in L.items():
        if k.startswith("CONFDEEP by"):
            t = parse(v)
            ax = k.split("(")[0].replace("CONFDEEP by", "").strip()
            cells = []
            for b, d in sorted(t.items()):
                cells.append(f"{b}:{d['made']}/{d['used']}u/{d['fix'] + d['prune']}x/{d['aused'] + d['oused']}lost")
            print(f"  {ax}: " + "  ".join(cells))
