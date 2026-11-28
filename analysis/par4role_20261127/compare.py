"""既定 (logs/<問題>_t4.*) と試作 (logs/<問題>_t4_<値>.*) を並べる: python compare.py <値> > compare_<値>.txt
(.gz は先に解いておく: gunzip -k logs/*.gz)。全体の秒、根の秒と本数、木の秒と本数、根の遊び (スレッド秒)、PAR4ROLE の数え。
区間の分け方は phases.py と同じ (根 = 最後の「TRSTAT: tree start」まで)"""
import json, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent; L = D / "logs"
V = sys.argv[1]
def phases(name, th):
    rows = [json.loads(l) for l in open(L / f"{name}.cpu.jsonl")]
    log = (L / f"{name}.log").read_text(errors="replace")
    m = re.search(r"solve returned \w+ after ([\d.]+)s", log); T = float(m.group(1))
    ivs = [(a["t"], b["t"] - a["t"], sum(b["cpu"][k] - a["cpu"].get(k, 0.0) for k in b["cpu"])) for a, b in zip(rows, rows[1:])]
    last = max((s + dt for s, dt, c in ivs if c > 0.05 * dt), default=ivs[-1][0]); off = last - T
    ts = [float(x) for x in re.findall(r"TRSTAT: tree start outer ([\d.]+)", log)]
    tr = ts[-1] if ts else T  # 最後の再スタートの後の木の始まり (外側の時計。「tree search with」の秒は再スタートの後の内側の時計なので使わない)
    def seg(a, b):
        w = sum(dt for s, dt, c in ivs if a <= s - off < b); c = sum(c for s, dt, c in ivs if a <= s - off < b)
        return w, (c / w if w else 0.0)
    idle = [float(x) for x in re.findall(r"tid \d+ ready [\d.]+s first [\d.]+s end [\d.]+s idle ([\d.]+)s", log)]
    tend = [float(x) for x in re.findall(r"tid \d+ ready [\d.]+s first [\d.]+s end ([\d.]+)s", log)]
    sb = re.findall(r"TREEPAR: spawn at ([\d.]+)s tree start ([\d.]+)s nodes \d+ sb ([\d.]+)s", log)
    return T, tr, seg(0, tr), seg(tr, T + 1), idle, tend, sb
P = ["nu25-pr12", "exp-1-500-5-5", "neos-1122047", "neos-1171448", "mzzv42z", "dano3_3", "mc11", "physiciansched6-2", "swath3", "supportcase7", "mcsched", "CMS750_4", "netdiversion"]
def stat(log):
    m = re.search(r"\"status\": \"(\w+)\"", log)
    return re.search(r"solve returned (\w+)", log).group(1)
print(f"{'問題':20} {'既定 秒':>8} {V + ' 秒':>8} {'比':>5} | {'根 秒 既定→' + V:>16} {'本数':>9} | {'木 秒':>13} {'本数':>9} | {'根の遊び':>13} | PAR4ROLE (a b 解 採用)")
lr = []; idle = [0.0, 0.0]
for p in P:
    a, b = f"{p}_t4", f"{p}_t4_{V}"
    if not (L / f"{b}.log").exists(): continue
    ra, rb = phases(a, 4), phases(b, 4)
    la, lb = (L / f"{a}.log").read_text(errors="replace"), (L / f"{b}.log").read_text(errors="replace")
    sa, sb = re.search(r"solve returned (\w+)", la).group(1), re.search(r"solve returned (\w+)", lb).group(1)
    ta, tb = ra[0], rb[0]
    lr.append(math.log((tb + 1) / (ta + 1)))
    ia, ib = max(0, 4 - ra[2][1]) * ra[2][0], max(0, 4 - rb[2][1]) * rb[2][0]; idle[0] += ia; idle[1] += ib
    pr = re.search(r"PAR4ROLE: helpers a (\d+) b (\d+), helper solutions (\d+), accepted as incumbent (\d+)", lb)
    pr = " ".join(pr.groups()) if pr else "-"
    flag = "" if sa == sb else f"  状態 {sa}→{sb}"
    print(f"{p:20} {ta:8.1f} {tb:8.1f} {(tb + 1) / (ta + 1):5.2f} | {ra[1]:7.1f} → {rb[1]:6.1f} {ra[2][1]:4.1f}→{rb[2][1]:4.1f} | {ta - ra[1]:6.1f}→{tb - rb[1]:6.1f} {ra[3][1]:4.1f}→{rb[3][1]:4.1f} | {ia:6.0f}→{ib:6.0f} | {pr}{flag}")
print(f"\n{len(lr)} 問の時間の比 ({V} / 既定、shift 1 秒) の幾何平均: {math.exp(sum(lr) / len(lr)):.3f}。根の遊びの合計 (スレッド秒): {idle[0]:.0f} → {idle[1]:.0f}")
