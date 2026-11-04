"""logs58/*.txt (summ_log.py の要約) から、再スタートのあった問題について、設定ごとに再スタートの回数・最初の求解の根の終わりの値・
最後の求解の根の終わりの値 (背景カットの後、なければカットのループの後) を並べる。使い方: python summ_root.py [設定A 設定B]"""
import re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
a, b = (sys.argv[1:3] if len(sys.argv) > 2 else ("def", "rr"))
def parse(f):
    segs, cur = [], {}
    for l in open(f):
        if m := re.match(r"  \[(\d+)\] (root LP|cut loop ->|bg cuts ->) ([-0-9.e+inf]+)", l):
            k = int(m[1])
            while len(segs) <= k:
                segs.append({})
            segs[k][m[2]] = float(m[3])
    return segs
def end(s):
    return s.get("bg cuts ->", s.get("cut loop ->", s.get("root LP")))
probs = sorted({f.name.split("_", 1)[1][:-4] for f in D.glob("logs58/*.txt")})
print(f"| 問題 | {a} 再スタート | {a} 根 (最初 → 最後) | {b} 再スタート | {b} 根 (最初 → 最後) | 差 (最後) |")
print("|---|---|---|---|---|---|")
for p in probs:
    fa, fb = D / f"logs58/{a}_{p}.txt", D / f"logs58/{b}_{p}.txt"
    if not (fa.exists() and fb.exists()):
        continue
    sa, sb = parse(fa), parse(fb)
    if len(sa) <= 1 and len(sb) <= 1:
        continue
    ea, eb = end(sa[0]) if sa else None, end(sb[0]) if sb else None
    la, lb = end(sa[-1]) if sa else None, end(sb[-1]) if sb else None
    d = f"{lb - la:+.6g}" if la is not None and lb is not None else ""
    print(f"| {p} | {len(sa)-1} | {ea:.8g} → {la:.8g} | {len(sb)-1} | {eb:.8g} → {lb:.8g} | {d} |" if None not in (ea, la, eb, lb) else f"| {p} | {len(sa)-1} | | {len(sb)-1} | | |")
