"""時刻ごとの下界・暫定解・開いたノード・処理したノードを並べる。
使い方: python traj.py logs/<問題>_<名前>.log ... (ENOMOTO は `TREE4:` の tid 0 の行、HiGHS はログの表の行)。
再スタートのある回は、ENOMOTO の時刻は再スタートごとに 0 に戻るので、前の回の最後の時刻を足して通しにする"""
import re, sys
TS = [30, 60, 120, 180, 240, 300, 360, 420, 480, 540, 600]

def ours(path):
    rows = []; off = 0.0; last = 0.0
    for l in open(path, errors="replace"):
        if l.startswith("TREE4: ") and " tid 0 " in l:
            d = dict(re.findall(r"(\w+) (\S+)", l[7:]))
            t = float(d["t"])
            if t + 1.0 < last:  # 再スタート
                off += last
            last = t
            rows.append((off + t, float(d["glb"]), float(d["ub"]), int(d["gopen"]), int(d["gnodes"])))
    return rows

def highs(path):
    rows = []
    for l in open(path, errors="replace"):
        m = re.match(r"^ [A-Z ] +(\d+) +(\d+) +(\d+) +([\d.]+)% +(\S+) +(\S+) +(\S+) .* (\d+(?:\.\d+)?)s$", l.rstrip())
        if m:
            f = lambda v: float(v) if v not in ("inf", "-inf") else float(v.replace("inf", "inf"))
            rows.append((float(m.group(8)), f(m.group(5)), f(m.group(6)), int(m.group(2)), int(m.group(1))))
    return rows

for path in sys.argv[1:]:
    rows = highs(path) if "highs" in path.split("/")[-1] else ours(path)
    print(f"## {path}")
    print("| 秒 | 下界 | 暫定解 | 開いたノード | 処理したノード |")
    print("|---:|---:|---:|---:|---:|")
    for T in TS:
        r = [x for x in rows if x[0] <= T]
        if r:
            t, lb, ub, op, nd = r[-1]
            print(f"| {T} | {lb:.10g} | {ub:.10g} | {op} | {nd} |")
