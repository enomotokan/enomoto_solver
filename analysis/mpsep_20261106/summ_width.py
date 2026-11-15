"""count_width.jsonl の集計 (python3 summ_width.py > summ_width.txt): 58 問・Mittelmann 186 問で、絞った形の門 (`g`: 広さ ≥ max(20, 行 × 0.5)、`h`: 根の LP ≥ 0.5 秒
または行 ≥ 5000) を通る問題を数えて一覧にする。根のカットのループに届かなかった問題 (前処理・根で解けた、時間切れ、取り寄せの失敗) も数える。"""
import json, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = [json.loads(l) for l in open(D / "count_width.jsonl")]
for st in ("58", "mitt"):
    v = [r for r in R if r["set"] == st]
    ok = [r for r in v if "width" in r]
    err = collections.Counter(r.get("error", "")[:40] for r in v if "width" not in r)
    g = [r for r in ok if r["wide_g"]]
    h = [r for r in ok if r["heavy_h"]]
    gh = [r for r in g if r["heavy_h"]]
    print(f"== {st}: {len(v)} 問、根のカットのループに届いた {len(ok)} 問 (届かず: {dict(err)})")
    print(f"  g (広い) {len(g)} 問、h (重い) {len(h)} 問、g かつ h {len(gh)} 問")
    for r in sorted(g, key=lambda r: -r["width"] / max(1, r["rows"])):
        print(f"    {r['problem']:32s} 広さ {r['width']:7d} 行 {r['rows']:7d} 比 {r['width'] / max(1, r['rows']):6.2f} 根の LP {r['root_lp_secs']:7.3f} 秒 分数 {r['nfrac']:6d}{'  (h も)' if r['heavy_h'] else ''}")
