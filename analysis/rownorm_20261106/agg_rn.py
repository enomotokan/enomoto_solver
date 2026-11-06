"""bench58.jsonl の追加の集計 (ROWNORM): 向き直した行がある問題の数と SGM、最適値の食い違い・異常終了、解けたかが違う回、向き直した行の数と時間の表。"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "bench58.jsonl"):
    r = json.loads(l); R[(r["problem"], r["cfg"], r["rep"])] = r["result"]
ps = sorted({k[0] for k in R})
def t(res):
    ok = res.get("status") in ("optimal", "infeasible") and res.get("time", 99) <= 60; return res["time"] if ok else 60.0
fl = [p for p in ps if any((R[(p, "rownorm", k)].get("rownorm") or [0])[0] > 0 for k in (0, 1))]
def sgm(c, qs):
    v = [math.log(t(R[(p, c, k)]) + 10) for p in qs for k in (0, 1)]; return math.exp(sum(v) / len(v)) - 10
nf = [p for p in ps if p not in fl]
print(f"向き直した行がある問題 {len(fl)} 問: SGM 既定 {sgm('def', fl):.2f} → ROWNORM {sgm('rownorm', fl):.2f}")
print(f"ない問題 {len(nf)} 問: SGM 既定 {sgm('def', nf):.2f} → ROWNORM {sgm('rownorm', nf):.2f}")
for p in ps:
    for k in (0, 1):
        a, b = R[(p, "def", k)], R[(p, "rownorm", k)]
        oa, ob = a.get("objective"), b.get("objective")
        if t(a) < 60 and t(b) < 60 and oa is not None and ob is not None and abs(oa - ob) > 1e-6 * max(1, abs(oa)): print("最適値の食い違い", p, k, oa, ob)
        if "crash" in (a.get("status"), b.get("status")): print("異常終了", p, k)
        if (t(a) < 60) != (t(b) < 60): print("解けたかが違う", p, k, "既定", round(t(a), 1), "ROWNORM", round(t(b), 1))
print("\n| 問題 | ROWNORM が向き直した行 (前処理ごと) | 既定 回0/回1 | ROWNORM 回0/回1 | 生成元 既定 → ROWNORM |")
print("|---|---|---|---|---|")
for p in fl:
    print(f"| {p} | {R[(p, 'rownorm', 0)].get('rownorm')} | {t(R[(p, 'def', 0)]):.1f} / {t(R[(p, 'def', 1)]):.1f} | {t(R[(p, 'rownorm', 0)]):.1f} / {t(R[(p, 'rownorm', 1)]):.1f} | {R[(p, 'def', 0)].get('gens')} → {R[(p, 'rownorm', 0)].get('gens')} |")
