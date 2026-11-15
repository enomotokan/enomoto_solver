"""logs/<接頭辞>.jsonl (ab.py の出力) を問題ごとに off / on の時間 (求解の秒)・目的値・CPU 使用 (CPU 秒 / 壁時計) で並べる。使い方: python summ.py 接頭辞..."""
import json, sys, collections
from pathlib import Path
O = Path(__file__).resolve().parent / "logs"
rows = collections.defaultdict(lambda: {"off": [], "on": []})
for tag in sys.argv[1:]:
    for l in open(O / f"{tag}.jsonl"):
        r = json.loads(l)
        rows[r["problem"]][r["cfg"]].append(r)
f = lambda v: "-" if v is None else (f"{v:.6g}")
print("| 問題 | 既定 (off) の秒 | 試作 (on) の秒 | off の目的値 | on の目的値 |")
print("|---|---|---|---|---|")
allo, alln = [], []
for p, d in rows.items():
    to = [r["result"]["time"] for r in d["off"]]; tn = [r["result"]["time"] for r in d["on"]]
    allo += to; alln += tn
    print(f"| {p} | {' / '.join(f'{t:.2f}' for t in to)} | {' / '.join(f'{t:.2f}' for t in tn)} | {' / '.join(f(r['result'].get('objective')) for r in d['off'])} | {' / '.join(f(r['result'].get('objective')) for r in d['on'])} |")
for name, a in (("off", allo), ("on", alln)):
    print(f"{name}: {len(a)} 回、61 秒以上 {sum(t >= 61 for t in a)} 回、最大 {max(a):.2f} 秒、上限からの超過の平均 {sum(t - 60 for t in a) / len(a):.2f} 秒")
cpu = [r["cpu"] / r["wall"] for d in rows.values() for c in d.values() for r in c]
print(f"CPU 使用 (CPU 秒 / 壁時計、Python の起動・読み込みを含む): {min(cpu):.2f}〜{max(cpu):.2f}")
