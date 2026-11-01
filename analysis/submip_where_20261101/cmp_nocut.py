"""既定 (logs) とサブ MIP の根のカットを抜いた形 (logs_nocut、ENOMOTO_MIP_SUBMIP_HIGHS=-sc) の比べ。使い方: python cmp_nocut.py > cmp_nocut.txt"""
import json, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze import parse, kind_of
D = Path(__file__).resolve().parent


def stat(f):
    subs, by, res = parse(f)
    d1 = [s for s in subs.values() if s["depth"] == 1]
    acc = sum(1 for s in d1 if s["acc"])
    secs = sum(s["end"]["secs"] for s in d1 if s["end"])
    nodes = sum(s["end"]["nodes"] for s in d1 if s["end"])
    Mev = by.get(0, [])
    t_best = None
    if Mev:
        zb = min(e["z"] for e in Mev)
        t_best = min(e["T"] for e in Mev if e["z"] <= zb + 1e-6 * (1 + abs(zb)))
    return res, len(d1), acc, secs, nodes, t_best


print("| 問題 | 既定: 状態 | 秒 | 目的値 | 深さ 1 の本数 / acc | 深さ 1 の秒 | ノード | 最終解の時刻 | -sc: 状態 | 秒 | 目的値 | 本数 / acc | 深さ 1 の秒 | ノード | 最終解の時刻 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for f in sorted((D / "logs_nocut").glob("*.log*")):
    p = f.name.split(".log")[0]
    out = [p]
    for g in (D / "logs" / f.name, f):
        res, n, a, s, nd, tb = stat(g)
        out += [res["status"] if res else "?", f"{res['time']:.1f}" if res else "?", f"{res['objective']}" if res else "?", f"{n} / {a}", f"{s:.1f}", str(nd), f"{tb:.1f}" if tb is not None else "-"]
    print("| " + " | ".join(out) + " |")
