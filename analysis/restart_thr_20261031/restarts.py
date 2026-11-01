"""blogs/ のログから、問題・回ごとに設定 (引数、既定 r,r_f01) の初回の判定 (新しい固定 / 整数列) と再スタートの回数・結果を並べる。
初回の固定が整数列の 1% 未満だった問題 (FIRST_FAC=0.01 が働く問題) だけを出す。全部出すには 2 つ目の引数に all。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
CF = sys.argv[1].split(",") if len(sys.argv) > 1 else ["r", "r_f01"]
ALL = len(sys.argv) > 2 and sys.argv[2] == "all"
info = {}
for f in (D / "blogs").glob("*.log"):
    stem, rep = f.stem.rsplit("_", 1)
    m = re.match(r"(.+?)_((?:def|r)(?:_[a-z0-9]+)*)$", stem)
    if not m or m.group(2) not in CF:
        continue
    t = f.read_text(errors="replace")
    chk = re.search(r"restart check: newly fixed (\d+), integer columns (\d+)", t)
    nrs = len(re.findall(r"MIP: restart \d+ \(\d+ of \d+ integer columns newly fixed", t))
    js = [l for l in t.splitlines() if l.startswith("{")]
    res = json.loads(js[-1]) if js else {}
    ok = res.get("status") == "optimal" and res.get("time", 99) <= 60
    info[(m.group(1), int(rep), m.group(2))] = (int(chk.group(1)) if chk else -1, int(chk.group(2)) if chk else 0, nrs, f"{res['time']:.1f}" if ok else "T")
probs = sorted({k[0] for k in info})
for p in probs:
    ks = [k for k in info if k[0] == p]
    small = any(0 < info[k][0] < 0.01 * info[k][1] for k in ks)
    if not (small or ALL):
        continue
    for rep in sorted({k[1] for k in ks}):
        print(f"{p:24s} 回 {rep}  " + "  ".join(f"{c}: 固定 {info[(p, rep, c)][0]}/{info[(p, rep, c)][1]} 再スタート {info[(p, rep, c)][2]} {info[(p, rep, c)][3]}" for c in CF if (p, rep, c) in info))
