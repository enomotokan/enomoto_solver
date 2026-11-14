"""1 スレッドと 4 スレッドの揺れを並べる: python cmp.py
問題ごとに両方の解けた数・最大/最小・並べ替えだけ/種だけの最大/最小を 1 行に。全体では、種だけと並べ替えだけの最大/最小の幾何平均
(どちらも 3〜4 回なので回数の違いに注意: 並べ替えだけは 4 回、種だけは 3 回) と、解けた回の合計を出す。"""
import math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.argv = [sys.argv[0], str(D / "res_t1.jsonl")]
import summ  # noqa: E402  (表を印字するが、ここでは load/stats だけ使う)
def agg(f):
    rows = summ.load(f); out = {}
    for p, runs in rows.items():
        out[p] = summ.stats(runs, next(iter(runs.values()))["tl"])[2]
    return out
a, b = agg(D / "res_t1.jsonl"), agg(D / "res_t4.jsonl")
gm = lambda xs: math.exp(sum(math.log(x) for x in xs) / len(xs))
print("\n\n# 1 スレッドと 4 スレッド\n")
for name, d in (("1 スレッド", a), ("4 スレッド", b)):
    ps = [p for p in d if d[p]["tmax"] > 1 and d[p]["solved"] > 0]  # 全部 1 秒未満・全部時間切れの問題は除く
    print(f"{name}: 解けた回 {sum(d[p]['solved'] for p in d)}/{sum(d[p]['n'] for p in d)}、"
          f"問題 {len(ps)} (1 回でも解けて 1 秒を超える) で 最大/最小の幾何平均 全部 {gm([d[p]['mm'] for p in ps]):.3f}、"
          f"並べ替えだけ {gm([d[p]['mm_perm'] for p in ps]):.3f}、種だけ {gm([d[p]['mm_seed'] for p in ps]):.3f}")
print("\n| 問題 | 1T 解けた | 1T 最大/最小 (並べ替え/種) | 4T 解けた | 4T 最大/最小 (並べ替え/種) |\n|---|---|---|---|---|")
for p in sorted(a, key=lambda p: -max(a[p]["gsd"], b.get(p, a[p])["gsd"])):
    if p not in b: continue
    x, y = a[p], b[p]
    if max(x["mm"], y["mm"]) < 1.5 and x["solved"] in (0, x["n"]) and y["solved"] in (0, y["n"]): continue
    print(f"| {p} | {x['solved']}/{x['n']} | {x['mm']:.2f} ({x['mm_perm']:.2f}/{x['mm_seed']:.2f}) | {y['solved']}/{y['n']} | {y['mm']:.2f} ({y['mm_perm']:.2f}/{y['mm_seed']:.2f}) |")
