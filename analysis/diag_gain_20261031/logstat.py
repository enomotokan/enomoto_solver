"""ログから得の仕組みの数字を抜き出す: 最終の暫定解を見つけたヒューリスティクスとその時刻 (再スタート後の時刻は再スタート後の経過秒)、
根の RINS のサブ MIP の結果 (physiciansched6-2)、根の LP face のサブ MIP の結果 (h80x6320d)、裏のカットのラウンド数と下界 (misc07)、再スタートの回数。
使い方: python logstat.py <問題> [設定,...]"""
import re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
p = sys.argv[1]
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
for f in sorted(D.glob(f"{p}_*.log")):
    c = f.stem[len(p) + 1:].rsplit("_", 1)[0]
    if cfgs and c not in cfgs:
        continue
    t = f.read_text()
    inc = re.findall(r"new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*?)\]", t)
    last = inc[-1] if inc else None
    # 最終の値と同じ値を最初に出した行
    first_best = next((i for i in inc if last and abs(float(i[0]) - float(last[0])) <= 1e-6 * max(1, abs(float(last[0])))), None)
    rins = re.search(r"sub-MIP: (\d+) free columns of \d+, status (\w+), nodes (\d+), objective (\S+), ([\d.]+)s\nMIP: root heuristic RINS", t)
    lpf = re.search(r"parallel sub-MIP \(LP face\): status (\w+), nodes (\d+), objective Some\(([\d.]+)", t)
    bg = re.search(r"background cuts: (\d+) rounds, \d+ cut rows \(was \d+\), obj (\S+) -> (\S+)", t)
    rs = len(re.findall(r"MIP: restart \d+ \(", t))
    res = re.search(r'"status": "(\w+)".*?"time": ([\d.]+)', t)
    out = [f"{c:12s} {f.stem[-1]}", f"{res.group(1)[:4]} {float(res.group(2)):5.1f}s" if res else "?", f"再始動 {rs}"]
    if first_best:
        out.append(f"最良 {float(first_best[0]):.6g} @{first_best[2]}s n{first_best[1]} [{first_best[3]}]")
    if rins:
        out.append(f"RINS {rins.group(2)} n{rins.group(3)} {rins.group(5)}s")
    if lpf:
        out.append(f"LPface {lpf.group(1)} n{lpf.group(2)} {float(lpf.group(3)):.1f}")
    if bg:
        out.append(f"裏のカット {bg.group(1)}r {float(bg.group(2)):.6g}->{float(bg.group(3)):.6g}")
    print(" | ".join(out))
