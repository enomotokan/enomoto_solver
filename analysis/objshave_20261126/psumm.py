"""pharm.jsonl の集計: 試作が働いた (問題, 種) の対で、shave と既定の時間 (時間切れは 60 秒) の比を、shift 1 秒の SGM と問題単位の
ブートストラップ 95% 区間で出す。解けた数、目的値の食い違い (両方とも最適で値が違う)、全種で 1.25 倍超遅い問題も出す。
試作が働かない対は既定と同じ設定なので比に入れない (比 1)。python psumm.py > psumm.txt"""
import json, math, random
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "pharm.jsonl").read_text().splitlines()]
by = {}
for r in rows: by.setdefault((r["problem"], r["seed"]), {})[r["cfg"]] = r
def t(r): return r["result"].get("time", 60.0) if r["result"].get("status") == "optimal" else 60.0
pairs = [(p, s, d["shave"], d["def"]) for (p, s), d in by.items() if "def" in d and d["shave"]["active"]]
print(f"P 58 問 × 種 1〜3、4 スレッド・60 秒。試作が働いた対 {len(pairs)} / {sum(1 for d in by.values() if 'shave' in d)}")
solved = lambda c: sum(1 for _, _, a, b in pairs if (a if c == 'shave' else b)["result"].get("status") == "optimal")
print("働いた対の解けた数: shave", solved("shave"), "既定", solved("def"))
bad = [(p, s, a["result"].get("objective"), b["result"].get("objective")) for p, s, a, b in pairs
       if a["result"].get("status") == "optimal" and b["result"].get("status") == "optimal"
       and abs(a["result"]["objective"] - b["result"]["objective"]) > 1e-6 * max(1, abs(b["result"]["objective"]))]
print("食い違い", len(bad), bad)
probs = sorted({p for p, *_ in pairs})
lr = {p: [math.log((t(a) + 1) / (t(b) + 1)) for q, _, a, b in pairs if q == p] for p in probs}
def sgm(ps): 
    v = [x for p in ps for x in lr[p]]; return math.exp(sum(v) / len(v))
g = sgm(probs); random.seed(0)
bs = sorted(sgm([random.choice(probs) for _ in probs]) for _ in range(2000))
print(f"働いた対の SGM (shave / 既定): {g - 1:+.1%} [{bs[50] - 1:+.1%}, {bs[1949] - 1:+.1%}]  問題 {len(probs)}")
# 174 対全体に広げたとき (働かない対は比 1)
n_all = sum(1 for d in by.values() if 'shave' in d)
print(f"全 {n_all} 対に広げた SGM: {math.exp(sum(x for v in lr.values() for x in v) / n_all) - 1:+.1%}")
print("\n問題ごと (shave / 既定 の時間の比、種ごと、証明の回数):")
for p in probs:
    ps = [(s, t(a) / t(b), a["proofs"]) for q, s, a, b in pairs if q == p]
    allslow = len(ps) == 3 and all(x > 1.25 for _, x, _ in ps)
    print(f"  {p}: " + ", ".join(f"種{s} {x:.2f} ({k})" for s, x, k in ps) + ("  全種で 1.25 倍超" if allslow else ""))
