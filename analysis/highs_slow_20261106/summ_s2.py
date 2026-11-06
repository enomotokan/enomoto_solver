"""S2 の結果の表 (2026-11-06): S2/*.jsonl を問題・タグごとに並べ、ログから要点 (裏のカットの待ち、再スタート、木の中の再スタート、
最後の暫定解の出どころ) を足す。使い方: python analysis/highs_slow_20261106/summ_s2.py [問題]"""
import json, re, statistics, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "S2"
only = sys.argv[1] if len(sys.argv) > 1 else None
rows = {}
for f in sorted(D.glob("*.jsonl")):
    for l in f.read_text().splitlines():
        r = json.loads(l)
        if only and r["problem"] != only:
            continue
        rows.setdefault((r["problem"], f.stem), []).append(r)
for (p, tag), rs in sorted(rows.items()):
    ts = []
    print(f"## {p} {tag}")
    for r in rs:
        res = r["result"]
        t = res.get("time") or 0.0
        ok = str(res.get("status")).lower() == "optimal"
        ts.append(t if ok else float("inf"))
        extra = ""
        log = D / f"logs/{tag}_{p}_{r['seed']}.log"
        if r["solver"] == "ours" and log.exists():
            s = log.read_text()
            waits = [float(x) for x in re.findall(r"determ: waited ([0-9.]+)s", s)]
            rst = len(re.findall(r"MIP: restart [0-9]+ \(", s))
            trst = len(re.findall(r"MIP: (?:tree restart|TREE restart):", s))
            inc = re.findall(r"new incumbent ([-0-9.e+]+) at node [0-9]+ \(([0-9.]+)s\) \[([^\]]+)\]", s)
            last = inc[-1] if inc else None
            extra = f"bgwait {sum(waits):.1f}s ({'+'.join(f'{w:.1f}' for w in waits)}), restarts {rst}, tree restarts {trst}, last inc {last}"
        print(f"  seed {r['seed']}: {res.get('status')} {t:.1f}s nodes {res.get('nodes')} {extra}")
    fin = [x for x in ts if x != float("inf")]
    print(f"  -> solved {len(fin)}/{len(ts)}, median {statistics.median(ts):.1f}, max {max(ts):.1f}")
