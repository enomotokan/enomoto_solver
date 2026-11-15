"""58 問の比べ (run58.py の出力) の要約。使い方: python summ58.py p58_base_rsi.jsonl"""
import json, math, sys
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
by = {}
for r in rows:
    by.setdefault(r["problem"], {})[r["cfg"]] = r["result"]


def gap(res):
    z, b = res.get("objective"), res.get("bound")
    if z is None or b is None or not all(map(math.isfinite, (z, b))):
        return 1.0
    return abs(z - b) / max(abs(z), abs(b), 1e-9) if abs(z - b) > 1e-9 else 0.0


def sgm(v, s=1.0):
    return math.exp(sum(math.log(x + s) for x in v) / len(v)) - s


opt = {"base": 0, "rsi": 0}
tb, tr, gb, gr = [], [], 0.0, 0.0
diff = []
for p, d in by.items():
    if "base" not in d or "rsi" not in d:
        continue
    b, r = d["base"], d["rsi"]
    for c in ("base", "rsi"):
        opt[c] += d[c].get("status") == "optimal"
    t1, t2 = min(b.get("time") or 60, 60), min(r.get("time") or 60, 60)
    tb.append(t1); tr.append(t2)
    g1, g2 = gap(b), gap(r)
    gb += g1; gr += g2
    if b.get("status") != r.get("status") or abs(t1 - t2) > max(5, 0.3 * min(t1, t2)) or abs(g1 - g2) > 1e-3:
        diff.append((p, b.get("status"), round(t1, 1), round(g1, 4), r.get("status"), round(t2, 1), round(g2, 4)))
n = len(tb)
print(f"問題 {n}: optimal base {opt['base']} / rsi {opt['rsi']}; 時間 (60 秒で頭打ち) のシフト幾何平均 base {sgm(tb):.2f} / rsi {sgm(tr):.2f}; 主双対ギャップの和 base {gb:.4f} / rsi {gr:.4f}")
print("差のある問題 (状態が違う・時間が 5 秒かつ 3 割以上違う・ギャップが 1e-3 以上違う): 問題 base(状態 時間 ギャップ) rsi(状態 時間 ギャップ)")
for x in diff:
    print("  ", *x)
