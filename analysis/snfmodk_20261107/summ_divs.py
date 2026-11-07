"""不変因子 (素数べきの因子) の分布の集計: python summ_divs.py 入力.jsonl 回 → 問題ごと・全体の数 (分離の最初の 50 回の和)。
zerohalf の連立 (snf の記録) と MODKVB の連立 (snfv の記録) の両方"""
import sys
from collections import Counter
from summ_root import snfinfo, cls, R
rep = int(sys.argv[3]) if len(sys.argv) > 3 else 1
tot = {"c": Counter(), "v": Counter()}
probs = {"c": Counter(), "v": Counter()}
others = {"c": Counter(), "v": Counter()}
for p in R:
    for key, cfg in (("c", "snf"), ("v", "snfv")):
        si = snfinfo(p, cfg, rep)
        if not si:
            continue
        cc = Counter()
        for q, v in si["divs"].items():
            if key == "c" and isinstance(q, int):
                cc[cls(q)] += v; 
                if cls(q) == "other": others[key][q] += v
            if key == "v" and isinstance(q, tuple):
                cc[cls(q[1])] += v
                if cls(q[1]) == "other": others[key][q[1]] += v
        tot[key].update(cc)
        for k in cc:
            probs[key][k] += 1
        if sum(cc.values()):
            probs[key]["any"] += 1
        if key == "c" and si["cuts_new"]:
            probs[key]["newcuts"] += 1
for key in ("c", "v"):
    print(key, "因子の数 (素数 2357 p / そのべき pp / ほかの素数とそのべき other):", dict(tot[key]), " 問題の数:", dict(probs[key]))
    print("   ほかの素数の多いもの:", others[key].most_common(12))
