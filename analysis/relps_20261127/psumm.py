"""prun.jsonl の集計: (スレッド, 種) ごとに、u と既定の時間 (時間切れは 60 秒) の比を shift 1 秒の SGM と問題単位のブートストラップ 95% 区間で出す。
解けた数、目的値の食い違い (両方とも最適で値が違う)、1.25 倍超の差の問題も出す。python psumm.py > psumm.txt"""
import json, math, random
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "prun.jsonl").read_text().splitlines()]
by = {}
for r in rows: by.setdefault((r["th"], r["seed"]), {}).setdefault(r["problem"], {})[r["cfg"]] = r
def t(r): return r["result"].get("time", 60.0) if r["result"].get("status") == "optimal" else 60.0
allpairs = []
for (th, seed), d in sorted(by.items()):
    pairs = [(p, x["u"], x["def"]) for p, x in sorted(d.items()) if "u" in x and "def" in x]
    allpairs += [(th, seed) + q for q in pairs]
    if not pairs: continue
    lr = {p: math.log((t(a) + 1) / (t(b) + 1)) for p, a, b in pairs}
    ps = list(lr); g = math.exp(sum(lr.values()) / len(ps)); random.seed(0)
    bs = sorted(math.exp(sum(lr[random.choice(ps)] for _ in ps) / len(ps)) for _ in range(2000))
    sol = lambda i: sum(1 for q in pairs if q[i]["result"].get("status") == "optimal")
    print(f"{th} スレッド・種 {seed}: {len(pairs)} 問、SGM (u / 既定) {g - 1:+.1%} [{bs[50] - 1:+.1%}, {bs[1949] - 1:+.1%}]、解けた数 u {sol(1)} 既定 {sol(2)}")
    act = [p for p, a, b in pairs if (a["relps"] or [0, 0])[1] or (b["relps"] or [0, 0])[1]]
    print(f"  既定で強分枝していない信頼できない列を選んだ問題 {sum(1 for p, a, b in pairs if (b['relps'] or [0,0])[1])}、u で {sum(1 for p, a, b in pairs if (a['relps'] or [0,0])[1])}")
    for p, a, b in pairs:
        x = (t(a) + 1) / (t(b) + 1)
        if x > 1.25 or x < 0.8:
            print(f"  {p}: {x:.2f} (u {a['result'].get('status')} {t(a):.1f} 秒 {a['result'].get('nodes')} ノード {a['relps']} / 既定 {b['result'].get('status')} {t(b):.1f} 秒 {b['result'].get('nodes')} ノード {b['relps']})")
bad = [(th, s, p) for th, s, p, a, b in allpairs if a["result"].get("status") == "optimal" and b["result"].get("status") == "optimal"
       and abs(a["result"]["objective"] - b["result"]["objective"]) > 1e-6 * max(1, abs(b["result"]["objective"]))]
crash = [(th, s, p) for th, s, p, a, b in allpairs if "crash" in (a["result"].get("status"), b["result"].get("status"))]
print("目的値の食い違い", bad, "落ちた", crash)
