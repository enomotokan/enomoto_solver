"""rules.jsonl を表にする: 問題ごとに、規則を 1 つ切ったときの前処理後の行・列 (既定との差)。"""
import json, collections
from pathlib import Path
NAMES = {64: "強制行", 128: "強制列", 256: "自由列の代入", 512: "2 項等式", 1024: "従属な等式", 2048: "従属な自由列", 4096: "Aggregator",
         8192: "平行な行・列", 16384: "Sparsify", 32768: "Probing", 65536: "Enumeration", 131072: "双対の固定", 262144: "列の詰め込み", 524288: "最初の掃き出し"}
R = collections.defaultdict(dict)
for l in (Path(__file__).parent / "rules.jsonl").read_text().splitlines():
    r = json.loads(l)
    if "rows" in r: R[r["problem"]][r["off"]] = r
for p, d in R.items():
    b = d.get(0)
    if not b: continue
    s = [f"{NAMES[k]} {d[k]['rows'] - b['rows']:+d}/{d[k]['cols'] - b['cols']:+d}" for k in sorted(d) if k and (d[k]['rows'], d[k]['cols']) != (b['rows'], b['cols'])]
    print(f"{p}: 既定 {b['rows']}×{b['cols']} ({b['secs']}s) | " + ("、".join(s) or "どれを切っても同じ"))
