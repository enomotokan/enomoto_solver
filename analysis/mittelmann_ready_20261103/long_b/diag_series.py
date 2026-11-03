"""診断の回 (long_ourslog*.jsonl、log=True) の、全スレッドの済んだノード・開いたノードと RSS の推移を CSV にする (2026-11-03)。
使い方: python analysis/mittelmann_ready_20261103/long_b/diag_series.py [LOGDIR]
利用者向けの経過行 (`<秒>s <済んだノード> <開いたノード> <下界> <暫定解> <ギャップ>`、求解の時計) を 30 秒ごとに間引き、同じ求解の時計に
近い RSS の標本を当てる (RSS の標本は起動からの経過なので、求解の始め = 経過 − (wall − 求解の時間) で寄せる)。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
LOGDIR = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/claude-0/longb_logs")
PROG = re.compile(r"^\s+([0-9.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)")
for f in sorted(D.glob("long_ourslog*.jsonl")):
    for l in f.read_text().splitlines():
        r = json.loads(l); tag = r.get("tag", "")
        out = LOGDIR / f"ourslog{tag}_{r['problem']}_{int(r['time_limit'])}.out"
        prog = [m.groups() for m in map(PROG.match, out.read_text().splitlines()) if m]
        t_solve = r["result"].get("time") or (r["series"][-1][0] - 15)  # 落ちた回は解く時間が分からないので読み込みを ~15 秒とみる
        off = r["wall"] - t_solve
        ser = [s for s in r["series"] if s[1] > 0]
        rows, last = ["solve_s,nodes_done,open,rss_mb,bytes_per_open,lb,ub,gap"], -1e9
        for g in prog:
            t = float(g[0])
            if t - last < 30: continue
            last = t
            s = min(ser, key=lambda x: abs(x[0] - off - t))
            op = int(g[2])
            rows.append(f"{t:.0f},{g[1]},{op},{s[1]:.0f},{(s[1] * 2**20 / op) if op else 0:.0f},{g[3]},{g[4]},{g[5]}")
        name = f"diag{tag}_{r['problem']}_{int(r['time_limit'])}.csv"
        (D / name).write_text("\n".join(rows) + "\n")
        print(name, len(rows) - 1)
