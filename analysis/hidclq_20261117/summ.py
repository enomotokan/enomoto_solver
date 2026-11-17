"""隠れたクリーク型の行の数え上げの集計: python summ.py > summ.txt。問題ごとに行・非零、2 値列だけの行、(A) 係数が違うのにクリークと同じ行、
(B) 係数が同じで右辺/係数 > 1 の行、(C) 既にクリーク型、(P) 一部がクリークの行 (平均の k / 行の長さ)、LP の下界の伸び (相対) を並べる。"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / "res.jsonl") if l.strip()]
pat = re.compile(r"rows (\d+) nnz (\d+); binary rows (\d+) \(two-sided (\d+)\); hidden clique \(A, unequal coefs\) (\d+) \(nnz (\d+)\); equal coefs b/a > 1 \(B\) (\d+) \(nnz (\d+)\); clique form \(C\) (\d+); partial clique \(P\) (\d+) \(mean k ([0-9.]+) of len ([0-9.]+)\); root LP \S+; LP with (.*?) \(([0-9.]+)s\)")
def gain(s, name):
    m = re.search(re.escape(name) + r" \S+ \(gain (\S+) rel", s)
    return float(m.group(1)) if m else None
out = []
for r in rows:
    a = next((l for l in r["lines"] if "HIDCLQ:" in l), None)
    m = pat.search(a) if a else None
    if not m: out.append((r["problem"], None, None)); continue
    g = m.groups(); lp = g[12]
    out.append((r["problem"], g, (gain(lp, "AB"), gain(lp, "P"), gain(lp, "AB+P"))))
print(f"{len(rows)} 問 (HIDCLQ なし: {[p for p, g, _ in out if g is None]})\n")
print(f"{'問題':22s} {'行':>7s} {'非零':>9s} {'2値行':>7s} {'両側':>6s} | {'(A)':>6s} {'非零':>7s} {'(B)':>6s} {'(C)':>6s} {'(P)':>6s} {'k/長さ':>11s} | {'伸び AB':>9s} {'P':>9s} {'AB+P':>9s}")
f = lambda x: "-" if x is None else f"{x:9.2e}"
for p, g, gs in sorted(out, key=lambda t: -(int(t[1][4]) + int(t[1][6]) if t[1] else -1)):
    if g is None: print(f"{p:22s} (HIDCLQ なし)"); continue
    print(f"{p:22s} {g[0]:>7s} {g[1]:>9s} {g[2]:>7s} {g[3]:>6s} | {g[4]:>6s} {g[5]:>7s} {g[6]:>6s} {g[8]:>6s} {g[9]:>6s} {g[10]+'/'+g[11]:>11s} | {f(gs[0])} {f(gs[1])} {f(gs[2])}")
print("\n例 (A):")
for r in rows:
    a = next((l for l in r["lines"] if "HIDCLQ:" in l), "")
    if "e.g." in a: print(f"  {r['problem']}: {a.split('e.g. ', 1)[1][:300]}")
