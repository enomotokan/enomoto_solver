"""改良案の比較: 基準の結果 (JSON Lines) と各案の結果を比べ、解けた数・幾何平均・退行を出す。

    python scripts/crossover_bench/compare_ab.py benchmarks/crossover/ab/base.jsonl benchmarks/crossover/ab/gondzio2.jsonl ...

退行 = 基準で解けた問題が解けなくなった、または内点法が収束せず二段解法で解き直すようになった問題。
"""
from __future__ import annotations

import json
import math
import sys


def load(path):
    out = {}
    for line in open(path):
        r = json.loads(line)
        ev = dict(r.get("events") or [])
        r["fallback"] = "crossover_fallback" in ev
        out[(r["set"], r["problem"])] = r
    return out


def gm(ts, shift=0.0):
    return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift


def main():
    base = load(sys.argv[1])
    limit = 600.0
    print("| 案 | 集合 | 問題数 | 解けた数 | 幾何平均 (秒) | 10 秒ずらした幾何平均 | 基準との比 (幾何平均) | 二段解法へ戻した数 | 退行 |")
    print("|---|---|---|---|---|---|---|---|---|")
    for path in sys.argv[1:]:
        cur = load(path)
        name = path.rsplit("/", 1)[-1].removesuffix(".jsonl")
        for st in ["netlib", "kennington", "all"]:
            keys = [k for k in base if k in cur and (st == "all" or k[0] == st)]
            if not keys:
                continue
            t = [cur[k]["time"] if cur[k]["solved"] else limit for k in keys]
            tb = [base[k]["time"] if base[k]["solved"] else limit for k in keys]
            ratio = gm([a / b for a, b in zip(t, tb)])
            reg = [k[1] for k in keys if (base[k]["solved"] and not cur[k]["solved"]) or (cur[k]["fallback"] and not base[k]["fallback"])]
            print(f"| {name} | {st} | {len(keys)} | {sum(cur[k]['solved'] for k in keys)} | {gm(t):.4g} | {gm(t, 10):.4g} | {ratio:.3f} | "
                  f"{sum(cur[k]['fallback'] for k in keys)} | {', '.join(reg) if reg else 'なし'} |")


if __name__ == "__main__":
    main()
