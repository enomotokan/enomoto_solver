"""b58q (analysis/treeclose_20261114.md 9.33 節) の集計: python summ.py [NAME=nbq15]。問題 x 設定ごとに種ごとの (状態, 秒) と、
主のスレッドの開いたノードの最大・兄弟への戻り・基底の戻し (≒ 待ち行列から取り出し)・ノードの分離のラウンドを並べ、設定ごとの解けた数と
ずらし幾何平均 (ずらし 10 秒、時間切れは制限時間) を出す。"""
import glob, json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
NAME = sys.argv[1] if len(sys.argv) > 1 else "nbq15"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"{NAME}_s*.jsonl"))) for l in open(f) if l.strip()]
by = {}
for r in rows:
    by.setdefault(r["problem"], {}).setdefault(r["cfg"], {})[r["seed"]] = r["result"]
cfgs = ["d2q", "d2n", "nb1"]
tl = max((r.get("tl", 600) for r in rows), default=600)
for p in sorted(by):
    print(f"## {p}")
    for c in cfgs:
        v = by[p].get(c, {})
        cells = []
        for s, x in sorted(v.items()):
            st = x.get("stats", {})
            cells.append(f"s{s} {(x.get('status') or '?')[:4]} {x.get('time', 0):.0f}s open_max {st.get('open_max', '-')} sib {st.get('sibling_backtracks', '-')} restore {st.get('basis_restores', '-')} sep {st.get('node_cut_rounds', '-')} intervals {st.get('nb5_intervals', '-')} choose {st.get('nb5_choose_secs', '-')}s skipped {st.get('nb5_skipped', '-')} gp_best {st.get('nb5_best', '-')} last {st.get('nb5_last', '-')}")
        print(f"  {c:5} " + "  |  ".join(cells))
for c in cfgs:
    xs = [x for p in by for x in by[p].get(c, {}).values()]
    if not xs:
        continue
    ts = [min(tl, x.get("time", tl)) for x in xs]
    ok = sum((x.get("status") or "").lower() == "optimal" for x in xs)
    print(f"{c}: 解けた {ok}/{len(xs)}、ずらし幾何平均 {math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10:.1f} 秒")

# 対の比 (同じ問題・種で d2q / d2n、d2q / nb1、時間切れは制限時間、ずらし 10 秒の比の幾何平均)
for a, b in (("d2q", "d2n"), ("d2q", "nb1"), ("d2n", "nb1")):
    rs = [math.log((min(tl, by[p][a][s].get("time", tl)) + 10) / (min(tl, by[p][b][s].get("time", tl)) + 10))
          for p in by for s in by[p].get(a, {}) if s in by[p].get(b, {})]
    if rs:
        print(f"対の比 {a}/{b}: {math.exp(sum(rs) / len(rs)):.3f} ({len(rs)} 組、{a} が速い {sum(r < -0.05 for r in rs)}・遅い {sum(r > 0.05 for r in rs)})")
# GP の超パラメータの内訳 (d2q の全部の回の選び直しを足す)。ell は (PQ, SQ, u) の長さの尺度、noise は雑音
from collections import Counter
cnt = Counter()
for p in by:
    for s, x in by[p].get("d2q", {}).items():
        for k, n in (x.get("stats", {}).get("nb5_hyp_counts") or {}).items():
            cnt[k] += n
tot = sum(cnt.values())
if tot:
    print(f"d2q の GP 超パラメータ (選び直し {tot} 回): " + ", ".join(f"{k} {n / tot:.0%}" for k, n in cnt.most_common(8)))
    nz = Counter()
    for k, n in cnt.items():
        nz[k.split("~")[1]] += n
    print("  雑音: " + ", ".join(f"{k} {n / tot:.0%}" for k, n in nz.most_common()))
