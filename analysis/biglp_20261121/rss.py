"""logs/<問題>_<ソルバー>.rss (経過秒 RSS(MB) スレッド数) から、最大とその時刻、指定の時刻の RSS を出す。
使い方: python rss.py 問題 ソルバー [時刻...]"""
import sys
p, s = sys.argv[1], sys.argv[2]
R = [tuple(map(float, l.split())) for l in open(f"logs/{p}_{s}.rss") if l.strip()]
mx = max(R, key=lambda r: r[1])
at = lambda t: min(R, key=lambda r: abs(r[0] - t))
print(f"{p} {s}: 最大 {mx[1]:.0f} MB ({mx[0]:.0f} 秒)、スレッド数の最大 {max(r[2] for r in R):.0f}; " +
      "、".join(f"{float(t):.0f} 秒 {at(float(t))[1]:.0f}" for t in sys.argv[3:]))
