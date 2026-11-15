"""m600 (種 1) と m600s (種 2・3) を合わせた base2 対 tca の集計: python summ_m600s.py。
問題ごとに種ごとの (状態, 秒 / 最後の全体のギャップ) を並べ、解けた数・時間のずらし幾何平均 (ずらし 10 秒、時間切れは 600 秒) を出す。"""
import json, math
from pathlib import Path
import parse

D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in sorted(D.glob("res_m600_s*.jsonl")) + sorted(D.glob("res_m600s_s*.jsonl")) for l in f.read_text().splitlines() if l.strip()]
R = {}
for r in rows:
    R[(r["problem"], r["cfg"], r["seed"])] = r
def gap(r):
    tag = f"{r['problem']}_{r['cfg']}" + (f"_s{r['seed']}" if r["seed"] != 1 else "")
    d = parse.eno(str(D / "logs" / f"{tag}.log.gz"))
    if not d["pts"]:
        return float("nan")
    last = d["pts"][-1]
    den = abs(r["result"].get("objective") or 1) or 1
    return (last[2] - last[1]) / den if all(math.isfinite(x) for x in (last[1], last[2])) else float("inf")
probs = sorted({k[0] for k in R})
tot = {c: [0, 0.0, 0] for c in ("base2", "tca")}
win = [0, 0]
for p in probs:
    line = [f"{p:22}"]
    for c in ("base2", "tca"):
        cell = []
        for s in (1, 2, 3):
            r = R.get((p, c, s))
            if r is None:
                cell.append("-")
                continue
            st, t = r["result"].get("status"), min(600.0, r["result"].get("time", 600))
            tot[c][1] += math.log(t + 10)
            tot[c][2] += 1
            if st == "optimal":
                tot[c][0] += 1
                cell.append(f"{t:.0f}s")
            else:
                cell.append(f"{100 * gap(r):.2g}%")
        line.append(f"{c}: " + " / ".join(cell))
    for s in (1, 2, 3):
        a, b = R.get((p, "base2", s)), R.get((p, "tca", s))
        if a and b:
            ta, tb = min(600, a["result"].get("time", 600)), min(600, b["result"].get("time", 600))
            if ta < 590 or tb < 590:
                win[0 if tb < ta else 1] += 1
    print("  ".join(line))
for c, (n, lg, k) in tot.items():
    print(f"{c}: 解けた {n}/{k}、ずらし幾何平均 {math.exp(lg / k) - 10:.1f} 秒")
print(f"どちらかが解けた対: tca が速い {win[0]}、base2 が速い {win[1]}")
