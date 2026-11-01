"""ログから外側の求解の様子を集計する: 時間、再スタートの回数、根のヒューリスティクスの時間 (再スタートごと、ログの時計は再スタートで戻る)、
根のサブ MIP の時間の和、最終の値を最初に出した暫定解 (値・時刻・区間・ヒューリスティクス)。
使い方: python logstat.py <問題> [設定,...]"""
import re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
p = sys.argv[1]
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
def key(f):
    c, r = f.stem[len(p) + 1:].rsplit("_", 1)
    return (c, int(r))
for f in sorted(D.glob(f"{p}_*.log"), key=key):
    c, rep = key(f)
    if cfgs and c not in cfgs:
        continue
    s = f.read_text()
    # 区間 (0 = 最初の求解、k = k 回目の再スタートの後) ごとに分ける
    parts = re.split(r"^MIP: restart \d+ \(.*$", s, flags=re.M)
    rh = []
    best = None
    for k, t in enumerate(parts):
        rh += [float(x) for x in re.findall(r"^MIP: after root heuristics: .*\(([\d.]+)s\)", t, re.M)][:1]
        for v, n, tm, lab in re.findall(r"new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*?)\]", t):
            v = float(v)
            if best is None or v < best[0] - 1e-6 * max(1, abs(v)):
                best = (v, k, float(tm), lab)
    sub = [float(t) for t in re.findall(r"sub-MIP(?: \([^)]*\))?: (?:\d+ free columns of \d+, )?status \w+, nodes \d+, objective [^,]*(?:\([^)]*\))?, ([\d.]+)s", s)]
    res = re.search(r'"status": "(\w+)".*?"time": ([\d.]+)', s)
    out = [f"{c:12s} {rep}", f"{res.group(1)[:4]} {float(res.group(2)):5.1f}s" if res else "?", f"再始動 {len(parts) - 1}",
           "根ヒュ " + "+".join(f"{x:.1f}" for x in rh), f"サブMIP {sum(sub):5.1f}s"]
    if best:
        out.append(f"最良 {best[0]:.6g} 区間{best[1]} @{best[2]:.1f}s [{best[3]}]")
    print(" | ".join(out))
