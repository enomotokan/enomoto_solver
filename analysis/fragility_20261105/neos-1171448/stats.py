"""設定ごとの時間の中央値・最悪・平均 (log_<設定>_<種>.txt、既定は log_c<種>.txt)。使い方: python stats.py"""
import glob, json, os, re, statistics as st
d = os.path.dirname(os.path.abspath(__file__))
groups = {}
for f in glob.glob(os.path.join(d, "log_*.txt")):
    tag = os.path.basename(f)[4:-4]
    m = re.fullmatch(r"c(\d+)", tag) or re.fullmatch(r"([a-z]+)_(\d+)", tag)
    if not m: continue
    g = "def" if tag.startswith("c") and m.re.pattern.startswith("c") else m.group(1)
    try: r = json.loads(open(f).read().splitlines()[-1])
    except Exception: continue
    groups.setdefault(g, []).append(r["time"])
print("| 設定 | 回数 | 中央値 (秒) | 平均 | 最悪 | 最良 | 50 秒以上の回 |")
print("|---|---:|---:|---:|---:|---:|---:|")
for g, ts in sorted(groups.items(), key=lambda x: (x[0] != "def", x[0])):
    print(f"| {g} | {len(ts)} | {st.median(ts):.1f} | {st.mean(ts):.1f} | {max(ts):.1f} | {min(ts):.1f} | {sum(t >= 50 for t in ts)} |")
