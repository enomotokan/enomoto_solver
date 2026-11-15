"""bench58.jsonl の要約 (SGM は (t + 10) の幾何平均 - 10、解けなければ 60 秒)。"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in (D / "bench58.jsonl").read_text().splitlines():
    r = json.loads(l); R.setdefault(r["problem"], {})[r["cfg"]] = r
cfgs = ["def", "memcap"]
def t(r):
    res = r["result"]
    return min(res.get("time", 60), 60) if res.get("status") == "optimal" else 60.0
for c in cfgs:
    rs = [R[p][c] for p in R if c in R[p]]
    sgm = math.exp(sum(math.log(t(r) + 10) for r in rs) / len(rs)) - 10
    solved = sum(r["result"].get("status") == "optimal" for r in rs)
    print(f"{c}: n={len(rs)} SGM={sgm:.2f} solved={solved} rss_sum={sum(r['max_rss_mb'] for r in rs)/1024:.1f}GB rss_max={max(r['max_rss_mb'] for r in rs)}MB"
          f" memcap_level_reads>0: {sum(any('level-reads 0/0/0' not in m for m in r['memcap']) for r in rs)}")
print("\n両方の状態が違う・目的値が違う・時間の比が 1.25 を超える問題:")
for p, d in R.items():
    if len(d) < 2: continue
    a, b = d["def"], d["memcap"]
    ra, rb = a["result"], b["result"]
    ratio = (t(b) + 10) / (t(a) + 10)
    diff = ra.get("status") != rb.get("status") or (ra.get("status") == rb.get("status") == "optimal" and abs(ra["objective"] - rb["objective"]) > 1e-6 * (1 + abs(ra["objective"])))
    if diff or ratio > 1.25 or ratio < 0.8:
        print(f"  {p}: def {ra.get('status')} {t(a):.1f} {ra.get('objective')}  memcap {rb.get('status')} {t(b):.1f} {rb.get('objective')}  ratio {ratio:.2f}  rss {a['max_rss_mb']}/{b['max_rss_mb']}")
