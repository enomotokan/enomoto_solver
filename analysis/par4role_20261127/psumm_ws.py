"""pws_s1〜3.jsonl の集計: ws / 既定の時間 (時間切れは 60 秒) の比を、shift 1 秒の SGM と問題単位のブートストラップ 95% 区間で出す。
解けた数、目的値の食い違い (両方とも最適で値が違う)、落ちた回、全種で 1.25 倍超遅い問題、w が待ちを打ち切った回・s が解を見つけた回も出す。
python psumm_ws.py > psumm_ws.txt"""
import json, math, random
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for k in (1, 2, 3) if (D / f"pws_s{k}.jsonl").exists() for l in (D / f"pws_s{k}.jsonl").read_text().splitlines()]
by = {}
for r in rows: by.setdefault((r["problem"], r["seed"]), {})[r["cfg"]] = r
def t(r): return r["result"].get("time", 60.0) if r["result"].get("status") == "optimal" else 60.0
pairs = [(p, s, d["ws"], d["def"]) for (p, s), d in sorted(by.items()) if "ws" in d and "def" in d]
print(f"P 58 問 × 種 1〜3、4 スレッド・60 秒。対 {len(pairs)}。ビルド {sorted({r['commit'] for r in rows})}")
sol = lambda i: sum(1 for x in pairs if x[i]["result"].get("status") == "optimal")
print(f"解けた数: ws {sol(2)}、既定 {sol(3)}")
bad = [(p, s) for p, s, a, b in pairs if a["result"].get("status") == "optimal" and b["result"].get("status") == "optimal"
       and abs(a["result"]["objective"] - b["result"]["objective"]) > 1e-6 * max(1, abs(b["result"]["objective"]))]
crash = [(p, s, c) for p, s, a, b in pairs for c, r in (("ws", a), ("def", b)) if r["result"].get("status") == "crash"]
print("目的値の食い違い", bad, "落ちた回", crash)
probs = sorted({p for p, *_ in pairs})
lr = {p: [math.log((t(a) + 1) / (t(b) + 1)) for q, _, a, b in pairs if q == p] for p in probs}
def sgm(ps):
    v = [x for p in ps for x in lr[p]]; return math.exp(sum(v) / len(v))
g = sgm(probs); random.seed(0)
bs = sorted(sgm([random.choice(probs) for _ in probs]) for _ in range(2000))
print(f"SGM (ws / 既定): {g:.3f} [{bs[50]:.3f}, {bs[1949]:.3f}]  問題 {len(probs)}")
wc = [(p, s) for p, s, a, b in pairs if a.get("w_capped")]
sf = [(p, s, a["s"]) for p, s, a, b in pairs if a.get("s") and a["s"][1] > 0]
print(f"w が待ちを打ち切った回 {len(wc)}: {wc}")
print(f"s が解を見つけた回 {len(sf)}: {sf}")
print("\n問題ごと (ws / 既定 の時間の比、種ごと):")
for p in probs:
    ps = [(s, (t(a) + 1) / (t(b) + 1)) for q, s, a, b in pairs if q == p]
    flag = "  全種で 1.25 倍超遅い" if len(ps) == 3 and all(x > 1.25 for _, x in ps) else ("  全種で 0.8 倍未満" if len(ps) == 3 and all(x < 0.8 for _, x in ps) else "")
    print(f"  {p}: " + ", ".join(f"種{s} {x:.2f}" for s, x in ps) + flag)
