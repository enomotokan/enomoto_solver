"""feat.jsonl (根だけ) の特徴と ratio.py の比を並べる"""
import json, re, math, subprocess, sys
F = {}
for l in open("feat.jsonl"):
    r = json.loads(l); F[(r["problem"], r["tag"])] = r
rat = {}
for l in subprocess.run([sys.executable, "ratio.py"], capture_output=True, text=True).stdout.splitlines():
    m = re.search(r"平均 ([\d.]+)", l)
    if m: rat[l.split()[0]] = float(m.group(1))
pat = re.compile(r"SEP tableau ([\d.]+)s cands (\d+) \(rows (\d+) of (\d+).*mean agg nnz (\d+), LP rows (\d+) nnz (\d+), cols (\d+) light (\w+) submip (\w+) pass (\d+) fill_would (\d+)")
def st(r):
    ls = [pat.search(x) for x in r.get("tab", [])]
    ls = [m for m in ls if m and m.group(9) == "false" and m.group(10) == "false"]
    if not ls: return None
    t = sum(float(m.group(1)) for m in ls); rows = sum(int(m.group(3)) for m in ls)
    agg = sum(int(m.group(5)) * int(m.group(3)) for m in ls) / max(1, rows)
    fw = sum(int(m.group(12)) for m in ls) / max(1, rows)
    m0 = ls[0]
    return dict(nr=len(ls), t=t, agg=agg, fw=fw, m=int(m0.group(6)), nnz=int(m0.group(7)), n=int(m0.group(8)))
print(f"{'問題':22s} {'比':>5s} {'回':>3s} {'tab秒':>6s} {'agg':>6s} {'n':>6s} {'m':>6s} {'nnz/m':>6s} {'agg/n':>6s} {'agg/rl':>6s} {'fw':>5s}  根def→tab  時間def→tab")
rows = []
for p in sorted({k[0] for k in F}, key=lambda p: rat.get(p, 9)):
    a, b = F.get((p, "def")), F.get((p, "tab"))
    s = st(a) if a else None
    if not s: print(p, "なし"); continue
    rl = s["nnz"] / max(1, s["m"])
    print(f"{p:22s} {rat.get(p, float('nan')):5.2f} {s['nr']:3d} {s['t']:6.2f} {s['agg']:6.0f} {s['n']:6d} {s['m']:6d} {rl:6.1f} {s['agg']/s['n']:6.2f} {s['agg']/rl:6.1f} {s['fw']:5.2f}  {a.get('root_bound')} → {b.get('root_bound') if b else None}  {a.get('time',0):.1f}→{(b or {}).get('time',0):.1f}")
