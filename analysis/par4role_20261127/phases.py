"""logs/ の 1T と 4T を並べ、区間ごとの壁の秒・CPU の本数・TREEPAR の待ちを表にする: python phases.py > phases.txt"""
import json, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; L = D / "logs"
res = {}
for l in open(D / "batch.out"):
    if l.startswith("{"):
        r = json.loads(l); res[(r["problem"], r["th"])] = r["result"]
def phases(name, th):
    rows = [json.loads(l) for l in open(L / f"{name}.cpu.jsonl")]
    log = (L / f"{name}.log").read_text(errors="replace")
    m = re.search(r"solve returned \w+ after ([\d.]+)s", log); T = float(m.group(1))
    ivs = [(a["t"], b["t"] - a["t"], sum(b["cpu"][k] - a["cpu"].get(k, 0.0) for k in b["cpu"])) for a, b in zip(rows, rows[1:])]
    last = max((s + dt for s, dt, c in ivs if c > 0.05 * dt), default=ivs[-1][0]); off = last - T
    ts = [float(x) for x in re.findall(r"TRSTAT: tree start outer ([\d.]+)", log)]
    tr = ts[-1] if ts else T  # 最後の再スタートの後の木の始まり (外側の時計。「tree search with」の秒は再スタートの後の内側の時計なので使わない)
    def seg(a, b):
        w = sum(dt for s, dt, c in ivs if a <= s - off < b); c = sum(c for s, dt, c in ivs if a <= s - off < b)
        return w, (c / w if w else 0.0)
    idle = [float(x) for x in re.findall(r"tid \d+ ready [\d.]+s first [\d.]+s end [\d.]+s idle ([\d.]+)s", log)]
    tend = [float(x) for x in re.findall(r"tid \d+ ready [\d.]+s first [\d.]+s end ([\d.]+)s", log)]
    sb = re.findall(r"TREEPAR: spawn at ([\d.]+)s tree start ([\d.]+)s nodes \d+ sb ([\d.]+)s", log)
    return T, tr, seg(0, tr), seg(tr, T + 1), idle, tend, sb
probs = sorted({p for p, th in res})
print("区間: 根 = 求解の始めから最後の「TRSTAT: tree start」まで (再スタートの前の木も根に入れる)、木 = その後。本数 = CPU 秒 / 壁の秒。")
print("待ち = TREEPAR の従のスレッドの idle 秒の合計 / (従の数 × 木の秒)。強分枝 = 木のスレッドを作る前に主が根のノードで強分枝に使った秒。\n")
print(f"{'問題':20} {'1T 秒':>7} {'4T 秒':>7} {'比':>5} | {'4T 根 秒':>8} {'本数':>4} | {'4T 木 秒':>8} {'本数':>4} {'待ち':>5} {'強分枝':>6} | {'1T 根 秒':>8} {'1T 木 秒':>8}")
for p in probs:
    if (p, 4) not in res: continue
    T4, tr4, (wr, cr), (wt, ct), idle, tend, sb = phases(f"{p}_t4", 4)
    t1 = res.get((p, 1), {}).get("time"); s1 = ""
    if (p, 1) in res:
        T1, tr1, (wr1, _), (wt1, _), *_ = phases(f"{p}_t1", 1); s1 = f"{tr1:8.1f} {T1 - tr1:8.1f}"
    wfrac = (sum(idle[1:]) / (3 * max(tend[0], 1e-9))) if len(idle) >= 4 and tend else float("nan")
    sbs = float(sb[-1][2]) if sb else float("nan")
    t1s = f"{t1:7.1f}" if t1 else "      -"; rat = f"{(t1 + 1) / (T4 + 1):5.2f}" if t1 else "    -"
    print(f"{p:20} {t1s} {T4:7.1f} {rat} | {tr4:8.1f} {cr:4.1f} | {T4 - tr4:8.1f} {ct:4.1f} {wfrac:5.0%} {sbs:6.1f} | {s1}")
print("\n遊んでいたスレッド秒 (4 本 − 本数) × 秒 の、根と木の内訳 (4T):")
tr_, tt_ = 0.0, 0.0
for p in probs:
    if (p, 4) not in res: continue
    T4, tr4, (wr, cr), (wt, ct), *_ = phases(f"{p}_t4", 4)
    a, b = max(0, 4 - cr) * wr, max(0, 4 - ct) * wt; tr_ += a; tt_ += b
    print(f"  {p:20} 根 {a:7.1f}  木 {b:7.1f}  (根が {a / max(a + b, 1e-9):4.0%})")
print(f"  合計: 根 {tr_:.0f}、木 {tt_:.0f} (根が {tr_ / (tr_ + tt_):.0%})")
