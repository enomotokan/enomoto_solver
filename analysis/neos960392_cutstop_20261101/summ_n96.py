"""neos-960392 の設定ごとのまとめ: 解けた回数、道 (LP に残ったカット・RENS の自由な列) の内訳、解けた回の時間、
カットのループの長さ (選択の比べの終わり → ループの終わり、clique の fallback の判定の分は除く)。使い方: python summ_n96.py"""
import re, subprocess, sys
from collections import Counter
from pathlib import Path
D = Path(__file__).resolve().parent
out = subprocess.run([sys.executable, str(D / "analyze.py"), "neos-960392"], capture_output=True, text=True).stdout.splitlines()[2:]
by = {}
for line in out:
    c = [x.strip() for x in line.strip("|").split("|")]
    by.setdefault(c[0], []).append(c)
print("| 設定 | 解けた | 解けた回の時間 | 道 (LP のカット・RENS の列) の内訳 | カットのループの長さ (秒) |")
print("|---|---|---|---|---|")
for cfg, rows in by.items():
    ok = [r for r in rows if r[2] == "optimal"]
    paths = Counter(f"{r[8]}・{r[11]}" for r in rows)
    lens = []
    for r in rows:
        m = re.match(r"([\d.]+)→([\d.]+)", r[10])
        if m:
            lens.append(float(m.group(2)) - float(m.group(1)))
    print(f"| {cfg} | {len(ok)}/{len(rows)} | {'・'.join(r[4] for r in ok) or '—'} | {'、'.join(f'{k} ×{v}' for k, v in paths.most_common())} | {'・'.join(f'{x:.1f}' for x in lens) or '—'} |")
