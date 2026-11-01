"""解を見つけた根のサブ MIP が、最後に良くなったのが何ノード目かを推定する (def のログ)。
成果連動の打ち切り (solver.rs の submip_outcome_stop) は、最後に良くなったノード k から max(k, 50) 進むと止めるので、
状態 NodeLimit で止まったノード数 n から k = n - 50 (50 <= n < 100) か k = n / 2 (n >= 100) と推定する
(ノード上限 500・停滞の打ち切り・時間切れで止まったものは区別できないので、n < 50 と 500 は別に数える)。
Optimal・Infeasible (木を解き切った) はノード数だけ出す。使い方: python impnode.py [設定 (既定 def)]"""
import sys
from collections import Counter
from summ import D, parse
cfg = sys.argv[1] if len(sys.argv) > 1 else "def"
for f in sorted(D.glob(f"*_{cfg}_*.log")):
    p, c, r = f.stem.rsplit("_", 2)
    if c != cfg: continue
    res, afts, subs = parse(f)
    ks = []
    for seg, name, st, n, found, t, kind in subs:
        if not found: continue
        if st == "NodeLimit" and 50 <= n < 100: ks.append(f"{n-50}")
        elif st == "NodeLimit" and 100 <= n < 500: ks.append(f"~{n//2}")
        elif st == "NodeLimit" and n >= 500: ks.append("上限")
        elif st == "NodeLimit": ks.append(f"<{n}?")
        else: ks.append(f"{st[:3]}{n}")
    print(f"{p:20s} {r}: {' '.join(ks)}")
