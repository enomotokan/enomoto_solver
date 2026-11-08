"""58 問 (r58.jsonl、plain 対 nl2、4 スレッド・60 秒) を analysis/benchlib.py の report で比べ、問題ごとの差の大きいものと誤答の疑いを出す。"""
import json, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import benchlib as bl
rows = bl.load([Path(__file__).resolve().parent / "r58.jsonl"])
R = bl.table(rows)
keys = list(R)
try:
    noise = bl.load_noise()
except Exception:
    noise = None
bl.report(R, "plain", "nl2", keys, noise=noise, label="NODELP2=pbxg")
# 問題ごと
print("\n大きい差 (|log((t2+10)/(t1+10))| > 0.3):")
import math
for k in sorted(keys):
    if "plain" in R[k] and "nl2" in R[k]:
        a, b = bl.t(R[k]["plain"]), bl.t(R[k]["nl2"])
        d = math.log((b + 10) / (a + 10))
        if abs(d) > 0.3:
            print(f"  {k}: {a:.1f} -> {b:.1f} ({d:+.2f})  nodes {R[k]['plain'].get('nodes')} -> {R[k]['nl2'].get('nodes')}")
# 誤答の疑い: ある回の下界が別の回の暫定解より (相対 1e-6 超) 良い
by = {}
for r in rows:
    res = r["result"]
    by.setdefault(r["problem"], []).append((res.get("objective"), res.get("bound"), r["cfg"], r["rep"]))
bad = 0
for p, v in by.items():
    objs = [o for o, _, _, _ in v if o is not None]
    bds = [b for _, b, _, _ in v if b is not None]
    if objs and bds:
        # 最小化として: 下界の最大 > 暫定解の最小 なら矛盾
        if max(bds) > min(objs) + 1e-6 * max(1.0, abs(min(objs))):
            bad += 1
            print("  誤答の疑い:", p, v)
print("誤答の疑い:", bad, "件")
