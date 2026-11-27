"""ログの行を、行の終わりの「(X.XXs)」の時刻で絞って出す: python win.py <名前> <開始秒> <終わり秒> [幅=170]"""
import re, sys
from pathlib import Path
n, a, b = sys.argv[1], float(sys.argv[2]), float(sys.argv[3]); w = int(sys.argv[4]) if len(sys.argv) > 4 else 170
for l in open(Path(__file__).resolve().parent / "logs" / f"{n}.log", errors="replace"):
    m = re.findall(r"\(([\d.]+)s\)", l)
    if m and a <= float(m[-1]) < b: print(f"{float(m[-1]):8.2f} {l.strip()[:w]}")
