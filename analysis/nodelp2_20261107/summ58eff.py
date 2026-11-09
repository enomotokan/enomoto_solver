"""r58eff.jsonl (4 スレッド・60 秒・種 1・2) の比べ: 今日の効率の直しを戻した形 (eff0) 対 今の既定 (plain)、今の既定 対 試作 ENOMOTO_MIP_DSE_INIT=umr (di3_p)。"""
import json, math, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import benchlib as bl
rows = bl.load([Path(__file__).resolve().parent / "r58eff.jsonl"])
R = bl.table(rows)
try:
    noise = bl.load_noise()
except Exception:
    noise = None
for A, B, lab in (("eff0", "plain", "効率の直し (戻した形 → 今の既定)"), ("plain", "di3_p", "試作 DSE_INIT=umr")):
    bl.report(R, A, B, list(R), noise=noise, label=lab)
    for k in sorted(R):
        if A in R[k] and B in R[k]:
            a, b = bl.t(R[k][A]), bl.t(R[k][B])
            d = math.log((b + 10) / (a + 10))
            if abs(d) > 0.3:
                print(f"    {k}: {a:.1f} -> {b:.1f} ({d:+.2f})")
by = {}
for r in rows:
    res = r["result"]
    by.setdefault(r["problem"], []).append((res.get("objective"), res.get("bound")))
bad = [p for p, v in by.items() if [o for o, _ in v if o is not None] and [b for _, b in v if b is not None]
       and max(b for _, b in v if b is not None) > min(o for o, _ in v if o is not None) + 1e-6 * max(1.0, abs(min(o for o, _ in v if o is not None)))]
print("誤答の疑い:", len(bad), bad)
