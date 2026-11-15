"""logs/ の各回 (利用者向けログ + 最後の行の JSON) から、時刻ごとの下界・ノード数・開いたノードを表にする。
使い方: python analysis/tree_close1_20261104/summ.py [問題]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
TIMES = [30, 60, 120, 200, 300, 600]
pat = re.compile(r"^\s+([0-9.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)")
for f in sorted(D.glob("*.out")):
    if f.name.startswith("H") or (len(sys.argv) > 1 and sys.argv[1] not in f.name):
        continue
    rows, res, offs, last = [], None, 0.0, 0.0
    for l in f.read_text().splitlines():
        if l.startswith("{"):
            res = json.loads(l)
        m = pat.match(l)
        if m:
            t = float(m.group(1))
            if t + 1e-9 < last:  # 再スタートの後は時刻が 0 から数え直される
                offs += last
            last = t
            rows.append((offs + t, int(m.group(2)), int(m.group(3)), m.group(4)))
    cells = []
    for T in TIMES:
        r = [x for x in rows if x[0] <= T]
        cells.append(f"{float(r[-1][3]):.6g}/{r[-1][1]}" if r and r[-1][3] != "-" else "-")
    print(f.stem, " | ".join(cells), "| final", None if res is None else (res["status"], res["bound"], res["objective"], res["nodes"]))
