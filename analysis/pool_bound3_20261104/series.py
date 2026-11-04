"""診断のログ (*.err.gz) から POOL3_DIAG の行を時刻ごとに表にする。
使い方: python series.py <ログ.err.gz> [間隔の秒 (既定 60)]
列: 時刻・報告の下界 (reported)・数えた最小 (counted)・打ち切り値・共有の待ち行列の大きさ・その下界の分布 (全体の最小ちょうど /
ギャップの 1% 以内 / 10% / 50% / それより上)・各スレッドの下界 (s<i>)・直前の行からの処理ノードとそのうち下界が全体の最小のもの。"""
import gzip, re, sys
path = sys.argv[1]; step = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
rows = []
for l in gzip.open(path, "rt", errors="replace"):
    if not l.startswith("POOL3_DIAG"):
        continue
    t = float(re.search(r" t ([\d.]+)", l).group(1))
    rep = re.search(r"reported (\S+)", l).group(1); cnt = re.search(r"counted (\S+)", l).group(1)
    inc = re.search(r" inc (\S+)", l).group(1)
    nn = re.search(r"nodes since last (\d+) at the global min (\d+)", l)
    m = re.search(r"len (\d+) hist.*?\[([^\]]*)\]", l)
    ln, hist = (m.group(1), m.group(2).replace(" ", "")) if m else ("-", "-")
    sl = " ".join(x.split("/")[0] for x in re.findall(r" s\d+ (\S+)", l))
    rows.append((t, rep, cnt, inc, ln, hist, sl, nn.group(1), nn.group(2)))
nxt = 0.0
print("| 秒 | 報告の下界 | 数えた最小 | 打ち切り値 | 共有の待ち行列 | 分布 (=最小,1%,10%,50%,上) | 各スレッドの下界 | 処理ノード (10 秒) / うち最小 |")
print("|---:|---:|---:|---:|---:|---|---|---:|")
for i, r in enumerate(rows):
    if r[0] + 1e-9 >= nxt or i == len(rows) - 1:
        nxt = r[0] + step
        print(f"| {r[0]:.0f} | {r[1]} | {r[2]} | {r[3]} | {r[4]} | {r[5]} | {r[6]} | {r[7]} / {r[8]} |")
