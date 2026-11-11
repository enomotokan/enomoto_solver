"""IPMRC の対の比べ (analysis/ipmrc_20261111.md)。python cmp.py A,B f1.jsonl [f2.jsonl ...]
- 全体: analysis/benchlib.py の report (SGM・解けた回・対の比の 95% 区間)。
- 誤答: optimal の目的値を既知の最適値 (scripts/miplib2017-v31.solu の =opt=) と比べ、下界が既知の最適値を越えていないか。
- 最大 RSS の比の幾何平均、IPMRC の行 (内点法の双対を足した根の LP の数)。"""
import json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
A, B = sys.argv[1].split(",")
raw = [json.loads(l) for f in sys.argv[2:] for l in open(f) if l.strip()]
tmp = D / ".cmp_tmp.jsonl"
tmp.write_text("".join(json.dumps(r) + "\n" for r in raw if r.get("run", 0) == 0))
R = bl.table(bl.load([tmp]))
tmp.unlink()
keys = [k for k in R if A in R[k] and B in R[k]]
bl.report(R, A, B, keys, label=f"{A} → {B}")
solu = {}
for l in open(D.parents[1] / "scripts" / "miplib2017-v31.solu"):
    p = l.split()
    if len(p) >= 3 and p[0] == "=opt=":
        solu[p[1]] = float(p[2])
close = lambda a, b: abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))
bad = []
for r in raw:
    res, p = r["result"], r["problem"]
    o, b = res.get("objective"), res.get("bound")
    if res.get("status") == "optimal" and o is not None and p in solu and not close(o, solu[p]):
        bad.append((p, r["cfg"], r["rep"], "obj", o, solu[p]))
    if p in solu and o is not None and isinstance(b, (int, float)) and math.isfinite(b):
        s = solu[p]
        if (o >= b and b > s + 1e-6 * max(1, abs(s))) or (o < b and b < s - 1e-6 * max(1, abs(s))):
            bad.append((p, r["cfg"], r["rep"], "bound", b, s))
print(f"\n誤答・暫定解と下界の矛盾 (既知の最適値と比べて): {len(bad)} 件 (行 {len(raw)})")
for x in bad:
    print("  ", x)
rss = {}
for r in raw:
    if r.get("maxrss_kb"):
        rss.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = r["maxrss_kb"]
lr = [math.log(v[B] / v[A]) for v in rss.values() if A in v and B in v]
if lr:
    print(f"最大 RSS の比の幾何平均 ({B}/{A}): {math.exp(sum(lr) / len(lr)):.3f} ({len(lr)} 組)")
n_ip = sum(1 for r in raw if r["cfg"] == B for e in r.get("xolazy", []) if e["ev"] == "ipmrc")
print(f"{B} で内点法の双対を足した根の LP: {n_ip}")
