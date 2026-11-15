"""b58h (analysis/treeclose_20261114.md 9.15 節) の集計: python summ.py [NAME=nbv]。問題 x 設定ごとに種ごとの (状態, 秒) と、
主のスレッドの開いたノードの最大・兄弟への戻り・基底の戻し (≒ 待ち行列から取り出し)・ノードの分離のラウンドを並べ、設定ごとの解けた数と
ずらし幾何平均 (ずらし 10 秒、時間切れは制限時間) を出す。"""
import glob, json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
NAME = sys.argv[1] if len(sys.argv) > 1 else "nbv"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"{NAME}_s*.jsonl"))) for l in open(f) if l.strip()]
by = {}
for r in rows:
    by.setdefault(r["problem"], {}).setdefault(r["cfg"], {})[r["seed"]] = r["result"]
cfgs = ["nb1", "nb2", "nb3"]
tl = max((r.get("tl", 600) for r in rows), default=600)
for p in sorted(by):
    print(f"## {p}")
    for c in cfgs:
        v = by[p].get(c, {})
        cells = []
        for s, x in sorted(v.items()):
            st = x.get("stats", {})
            cells.append(f"s{s} {(x.get('status') or '?')[:4]} {x.get('time', 0):.0f}s open_max {st.get('open_max', '-')} sib {st.get('sibling_backtracks', '-')} restore {st.get('basis_restores', '-')} sep {st.get('node_cut_rounds', '-')} arms {st.get('nb_pulls', '-')}")
        print(f"  {c:5} " + "  |  ".join(cells))
for c in cfgs:
    xs = [x for p in by for x in by[p].get(c, {}).values()]
    if not xs:
        continue
    ts = [min(tl, x.get("time", tl)) for x in xs]
    ok = sum((x.get("status") or "").lower() == "optimal" for x in xs)
    print(f"{c}: 解けた {ok}/{len(xs)}、ずらし幾何平均 {math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10:.1f} 秒")
