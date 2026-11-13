"""58 問の比較の集計 (bench.jsonl): c12p / c の SGM の比 (ずらし 10 秒、解けなければ 60 秒) と、(問題, 種) の対を引き直す 95% 区間、
解けた数、目的値の食い違い (両方 optimal で相対 1e-6 超)、種をまたいで 10% 超遅い・速い問題、再スタートの回数が違う回。"""
import json, math, random
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in (D / "bench.jsonl").read_text().splitlines():
    if l.strip():
        r = json.loads(l); R[r["cfg"], r["problem"], r["rep"]] = r
A, B = "c", "c12p"
def t(r):
    res = r["result"]; return min(res["time"], 60.0) if res.get("status") == "optimal" else 60.0
pairs = sorted({(p, s) for (c, p, s) in R if (A, p, s) in R and (B, p, s) in R})
lr = [math.log((t(R[B, p, s]) + 10) / (t(R[A, p, s]) + 10)) for p, s in pairs]
def sgm(c): return math.exp(sum(math.log(t(R[c, p, s]) + 10) for p, s in pairs) / len(pairs)) - 10
random.seed(0)
bs = sorted(math.exp(sum(random.choice(lr) for _ in lr) / len(lr)) for _ in range(5000))
print(f"対 {len(pairs)}  SGM {A} {sgm(A):.2f} / {B} {sgm(B):.2f}  比 {math.exp(sum(lr)/len(lr)):.3f} [{bs[125]:.3f}, {bs[4875]:.3f}]")
for c in (A, B):
    print(c, "解けた", sum(R[c, p, s]["result"].get("status") == "optimal" for p, s in pairs),
          "crash", sum(R[c, p, s]["result"].get("status") in ("crash", None) for p, s in pairs))
bad = []
for p, s in pairs:
    a, b = R[A, p, s]["result"], R[B, p, s]["result"]
    if a.get("status") == b.get("status") == "optimal" and abs(a["objective"] - b["objective"]) > 1e-6 * max(1, abs(a["objective"])):
        bad.append((p, s, a["objective"], b["objective"]))
print("目的値の食い違い", bad)
byp = defaultdict(list)
for p, s in pairs:
    byp[p].append((t(R[A, p, s]), t(R[B, p, s]), [x[1] for x in R[A, p, s]["restarts"]], [x[1] for x in R[B, p, s]["restarts"]]))
print("種をまたいで 10% 超遅い:", [p for p, v in byp.items() if all((b + 10) > 1.1 * (a + 10) for a, b, _, _ in v)])
print("種をまたいで 10% 超速い:", [p for p, v in byp.items() if all((b + 10) * 1.1 < (a + 10) for a, b, _, _ in v)])
print("再スタートの違う回:")
for p, v in byp.items():
    if any(ra != rb for _, _, ra, rb in v):
        print(" ", p, " | ".join(f"{a:.1f} {ra} -> {b:.1f} {rb}" for a, b, ra, rb in v))
