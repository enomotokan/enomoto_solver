"""lpbench.py の 1 つの結果 (lpbench_<tag>.jsonl) で、2 つの設定の単体法の時間を比べる (10 節)。

python analysis/lpiter_20261127/lpcmp.py tag 基準の設定 比べる設定 [...]
- 両方が解けた問題について、反復数が同じかを数え、計時された単体法 (PROF_PHASES_EXT の wall) が 0.1 秒以上の問題で
  単体法の時間・FTRAN・BTRAN の比 (中央値どうしの比の幾何平均) を出す。問題ごとの比も出す。
"""
import json, math, statistics, sys
from pathlib import Path

D = Path(__file__).resolve().parent
OK = ("optimal", "infeasible", "unbounded")


def gm(x):
    return math.exp(sum(map(math.log, x)) / len(x)) if x else float("nan")


def main():
    tag, base, cfgs = sys.argv[1], sys.argv[2], sys.argv[3:]
    R = {}
    for l in (D / f"lpbench_{tag}.jsonl").read_text().splitlines():
        r = json.loads(l)
        R.setdefault(r["problem"], {}).setdefault(r["cfg"], []).append(r["result"])
    med = lambda xs, k: statistics.median(x["prof"].get(k, 0.0) for x in xs)
    for c in cfgs:
        n = same = 0
        rows = []
        for p, x in R.items():
            if base not in x or c not in x:
                continue
            a = [y for y in x[base] if y.get("status") in OK]
            b = [y for y in x[c] if y.get("status") in OK]
            if not a or not b:
                continue
            n += 1
            same += a[0]["prof"]["iters"] == b[0]["prof"]["iters"]
            if med(a, "wall_ms") >= 100:
                rows.append((p, med(b, "wall_ms") / med(a, "wall_ms"), med(b, "ftran") / med(a, "ftran"),
                             med(b, "btran(rho_p)") / med(a, "btran(rho_p)"), med(a, "wall_ms") / 1e3))
        print(f"{tag} {c}/{base}: 両方解けた {n} 問、反復同じ {same} 問、0.1 秒以上 {len(rows)} 問: 単体法 {gm([r[1] for r in rows]):.3f}"
              f" FTRAN {gm([r[2] for r in rows]):.3f} BTRAN {gm([r[3] for r in rows]):.3f}")
        for p, s, f, bt, t in sorted(rows):
            print(f"  {p:24s} 単体法 {s:.3f} FTRAN {f:.3f} BTRAN {bt:.3f} ({base} {t:.2f} 秒)")


if __name__ == "__main__":
    main()
