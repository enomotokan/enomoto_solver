"""c3 の遅れの部品の切り分けの表 (README 8 節): python analysis/combo3_20261126/s2/abltab.py → 標準出力。
既定・c3 は 240 問の種 1 の行 (res_s*.jsonl)、c2c・c3−X は res_abl.jsonl (同じ版 cd10d12f7 のビルド、種 1、600 秒)。
種 2 の既定・c3 (res_harm.jsonl) も並べる。秒 (* は時間切れ)。"""
import glob, json
from pathlib import Path
D = Path(__file__).resolve().parent
TL = 600.0
def t(x): return min(x.get("time") or TL, TL) if x.get("status") in ("optimal", "infeasible") else TL
R = {}
for g in glob.glob(str(D / "res_s*.jsonl")):
    for l in open(g): r = json.loads(l); R[(r["problem"], r["cfg"][-1], r["cfg"][:-3], 1)] = r["result"]
for f in ("res_harm.jsonl", "res_abl.jsonl"):
    for l in open(D / f): r = json.loads(l); R[(r["problem"], r["threads"], r["cfg"], r["seed"])] = r["result"]
def s(k):
    x = R.get(k)
    return "—" if x is None else f"{t(x):.0f}{'' if x.get('status') in ('optimal','infeasible') else '*'}"
print("| 問題 | スレッド | 既定 | c3 | 種 2 既定 / c3 | c2c | c3−nrap | c3−share | c3−enumt | c3−nolp |")
print("|---|---|---:|---:|---|---:|---:|---:|---:|---:|")
for l in open(D / "abl_plan.txt"):
    th, p = l.split()
    cells = [s((p, th, c, 1)) for c in ("def", "c3", "c2c", "c3-nrap", "c3-share", "c3-enumt", "c3-nolp")]
    print(f"| {p} | {th} | {cells[0]} | {cells[1]} | {s((p, th, 'def', 2))} / {s((p, th, 'c3', 2))} | " + " | ".join(cells[2:]) + " |")
