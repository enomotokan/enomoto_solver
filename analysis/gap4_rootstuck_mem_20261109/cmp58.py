"""GAP4: res58.jsonl (58 問・60 秒・種 1・2) を benchlib の report で比べる。最大 RSS の和・比、両方最適で目的値が違う組も出す。
使い方: python cmp58.py > cmp58.txt"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
rows = bl.load([D / "res58.jsonl"])
R = bl.table(rows)
keys = list(R)
for A, B in [("def", "g4"), ("def", "g4n"), ("g4", "g4n")]:
    bl.report(R, A, B, keys, noise=bl.load_noise(), label=f"{A}→{B}")
    print()
# 最大 RSS と目的値の食い違い
raw = [json.loads(l) for l in open(D / "res58.jsonl")]
M = {}
for r in raw:
    M.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = r
for A, B in [("def", "g4"), ("def", "g4n"), ("g4", "g4n")]:
    tot = [0, 0]; big = []; bad = []
    for k, v in M.items():
        if A in v and B in v:
            a, b = v[A].get("max_rss_mb", 0), v[B].get("max_rss_mb", 0)
            tot[0] += a; tot[1] += b
            if a > 800 and abs(b - a) > 0.2 * a:
                big.append((k, a, b))
            ra, rb = v[A]["result"], v[B]["result"]
            if ra.get("status") == "optimal" and rb.get("status") == "optimal":
                za, zb = ra.get("objective"), rb.get("objective")
                if za is not None and zb is not None and abs(za - zb) > 1e-6 * max(1, abs(za)):
                    bad.append((k, za, zb))
            for x, y in [(ra, rb), (rb, ra)]:
                # 片方の下界がもう片方の解より 1e-6 を超えて上なら矛盾
                if x.get("bound") is not None and y.get("objective") is not None and x["bound"] > y["objective"] + 1e-6 * max(1, abs(y["objective"])):
                    bad.append((k, "bound>obj", x["bound"], y["objective"]))
    print(f"{A}→{B}: 最大 RSS の和 {tot[0]} → {tot[1]} MB ({tot[1] / max(1, tot[0]) - 1:+.1%})、20% 超の差 (800 MB 超): {sorted(big)}")
    print(f"  両方最適で目的値が違う・下界と解の矛盾: {bad}")
