"""待ち行列に足す: python addq.py 設定,... 問題,... 種 [秒]  (設定は cfgs.txt)"""
import sys
from pathlib import Path
D = Path(__file__).resolve().parent
C = {l.split(":")[0]: l.split(":", 1)[1].strip() for l in (D / "cfgs.txt").read_text().splitlines() if l and not l.startswith("#")}
TL = {"physiciansched6-2": 150, "cod105": 300}
cfgs, probs, seed = sys.argv[1].split(","), sys.argv[2].split(","), sys.argv[3]
with open(D / "queue.txt", "a") as f:
    for p in probs:
        for c in cfgs:
            tl = sys.argv[4] if len(sys.argv) > 4 else TL.get(p, 600)
            f.write(f"python run1.py ours {p} {c} {seed} {tl} {C[c]}\n".replace("  ", " "))
