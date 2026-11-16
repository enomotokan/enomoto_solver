"""切り分け I の集計: python summI.py > summ_I.txt。問題ごとに設定別の時間 (種 1〜8、時間切れは 60 秒) とずらし幾何平均 (shift 1)、
cand1117 との比 (外すと速くなる鍵が原因) と cand1114 との比。"""
import json, glob, math
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_I_s*.jsonl"))) for l in open(f) if l.strip()]
CF = json.loads((D / "shards_I.json").read_text())["cfgs"]
def st(r): return str(r["result"].get("status") or "").lower()
def t(r): return min(r["result"]["time"], r["tl"]) if st(r) in ("optimal", "infeasible") else r["tl"]
print("行数", len(rows), "crash", sum(1 for r in rows if st(r) not in ("optimal", "infeasible", "time_limit")))
for p in ("10teams", "neos-827175"):
    R = {(r["cfg"], r["seed"]): r for r in rows if r["problem"] == p}
    seeds = sorted({s for c, s in R})
    def g(c):
        v = [math.log(t(R[(c, s)]) + 1) for s in seeds if (c, s) in R]
        return math.exp(sum(v) / len(v)) - 1 if v else float("nan")
    b17, b14 = g("cand1117"), g("cand1114")
    print(f"\n## {p} (種 {seeds})  設定: ずらし幾何平均 秒 / 対 cand1117 / 対 cand1114 / 種ごとの秒 (節点数)")
    for c in sorted(CF, key=g):
        cells = " ".join(f"{t(R[(c, s)]):.0f}({R[(c, s)]['result'].get('nodes')})" for s in seeds if (c, s) in R)
        print(f"  {c:20s} {g(c):6.1f}  {(g(c) + 1) / (b17 + 1):5.2f}  {(g(c) + 1) / (b14 + 1):5.2f}  {cells}")
