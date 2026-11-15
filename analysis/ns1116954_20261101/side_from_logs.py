"""side.sh のログ (logs/side/) から結果 (状態・最後の暫定解・時間) を拾って表にする (side.out の行の一部を失ったため、2026-11-02)。"""
import re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs" / "side"
rows = {}
for f in sorted(D.glob("*.log")):
    p, v, r = re.match(r"(.+)_(def|early)_r(\d)\.log", f.name).groups()
    s = f.read_text()
    m = re.findall(r"solve returned (\w+) after ([0-9.]+)s", s)
    inc = re.findall(r"new incumbent ([-0-9.e]+) at", s)
    early = "early FJ" in s
    rows.setdefault(p, {}).setdefault(v, []).append((r, m[-1][0] if m else "?", float(m[-1][1]) if m else None, float(inc[-1]) if inc else None, early))
print("| 問題 | 既定 (状態 時間 目的値) | 試作 early (状態 時間 目的値) |\n|---|---|---|")
for p, d in rows.items():
    fmt = lambda xs: " / ".join(f"{st} {t:.1f}s {z:g}" if t is not None and z is not None else f"{st}" for _, st, t, z, _ in sorted(xs))
    print(f"| {p} | {fmt(d.get('def', []))} | {fmt(d.get('early', []))} |")
print("早めの FJ が動いた回:", [f.name for f in sorted(D.glob('*.log')) if 'early FJ' in f.read_text()] or "なし")
