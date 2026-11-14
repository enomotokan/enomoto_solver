"""TREECLOSE の試しの比べ: python cmp.py res_X_s*.jsonl  (問題ごとに設定を並べる: 状態・秒・ノード・最後の全体のギャップ・開いたノード・葉の割合)。"""
import json, re, sys, gzip
from pathlib import Path
import parse

D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in sys.argv[1:] for l in open(f) if l.strip()]
by = {}
for r in rows:
    by.setdefault(r["problem"], []).append(r)
for p, rs in sorted(by.items()):
    print(f"## {p}")
    for r in sorted(rs, key=lambda r: r["cfg"]):
        rr = r["result"]
        tag = f"{p}_{r['cfg']}" + (f"_s{r['seed']}" if r["seed"] != 1 else "")
        lf = D / "logs" / f"{tag}.log.gz"
        d = parse.eno(str(lf))
        last = d["pts"][-1] if d["pts"] else None
        den = abs(rr.get("objective") or 1) or 1
        gap = (last[2] - last[1]) / den if last and abs(last[2]) != float("inf") and abs(last[1]) != float("inf") else float("nan")
        # 最後の TRSTAT の処理数と開いたノード
        tr = None
        for line in gzip.open(lf, "rt", errors="replace"):
            if line.startswith("TRSTAT: due"):
                tr = line
        q = re.search(r"nodes (\d+) open (\d+) q (\S+)", tr) if tr else None
        qs = f"TRSTAT nodes {q.group(1)} open {q.group(2)} q {q.group(3)}" if q else ""
        print(f"  {r['cfg']:6} {rr.get('status'):10} {rr.get('time', 0):6.0f}s nodes {rr.get('nodes')} gap {gap:.2e} open(last) {last[4] if last else '-'} {qs}")
