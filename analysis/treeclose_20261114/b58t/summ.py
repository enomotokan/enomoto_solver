"""b58t (analysis/treeclose_20261114.md 9.39 節) の集計: python summ.py [NAME=fs18]。問題 x 設定ごとに種ごとの (状態, 秒) と、
主のスレッドの開いたノードの最大・兄弟への戻り・基底の戻し (≒ 待ち行列から取り出し)・ノードの分離のラウンドを並べ、設定ごとの解けた数と
ずらし幾何平均 (ずらし 10 秒、時間切れは制限時間) を出す。"""
import glob, json, math, sys
sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parent))
from pathlib import Path
D = Path(__file__).resolve().parent
NAME = sys.argv[1] if len(sys.argv) > 1 else "fs18"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"{NAME}_s*.jsonl"))) for l in open(f) if l.strip()]
by = {}
for r in rows:
    by.setdefault(r["problem"], {}).setdefault(r["cfg"], {})[r["seed"]] = r["result"]
from cfgs import CFGS
cfgs = list(CFGS)
CTRL = ["base"]
tl = max((r.get("tl", 600) for r in rows), default=600)
for p in sorted(by):
    print(f"## {p}")
    for c in cfgs:
        v = by[p].get(c, {})
        cells = []
        for s, x in sorted(v.items()):
            st = x.get("stats", {})
            cells.append(f"s{s} {(x.get('status') or '?')[:4]} {x.get('time', 0):.0f}s open_max {st.get('open_max', '-')} sib {st.get('sibling_backtracks', '-')} restore {st.get('basis_restores', '-')} sep {st.get('node_cut_rounds', '-')} d {st.get('dfsbb_d', '-')} L {st.get('dfsbb_l', '-')} leafdepth {st.get('leaf_depth', '-')} fjsub {st.get('fjsub', '-')} plunges {st.get('dfsbb_plunges', '-')} wcut {st.get('dfsbb_wcut', '-')} leaf {st.get('dfsbb_leaf', '-')}")
        print(f"  {c:5} " + "  |  ".join(cells))
for c in cfgs:
    xs = [x for p in by for x in by[p].get(c, {}).values()]
    if not xs:
        continue
    ts = [min(tl, x.get("time", tl)) for x in xs]
    ok = sum((x.get("status") or "").lower() == "optimal" for x in xs)
    print(f"{c}: 解けた {ok}/{len(xs)}、ずらし幾何平均 {math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10:.1f} 秒")

# 対の比 (同じ問題・種、時間切れは制限時間、ずらし 10 秒の比の幾何平均) と 95% 区間 (組の再標本 4000 回)
import random
random.seed(1)
for a in [c for c in cfgs if c not in CTRL]:
    for b in CTRL:
        rs = [math.log((min(tl, by[p][a][s].get("time", tl)) + 10) / (min(tl, by[p][b][s].get("time", tl)) + 10))
              for p in by for s in by[p].get(a, {}) if s in by[p].get(b, {})]
        if not rs:
            continue
        ms = sorted(sum(random.choice(rs) for _ in rs) / len(rs) for _ in range(4000))
        print(f"対の比 {a}/{b}: {math.exp(sum(rs) / len(rs)):.3f} [{math.exp(ms[100]):.3f}, {math.exp(ms[3899]):.3f}] ({len(rs)} 組、{a} が速い {sum(r < -0.05 for r in rs)}・遅い {sum(r > 0.05 for r in rs)})")
# 潜りの統計 (設定ごとに合計)
for c in [c for c in cfgs if c != "nb1"]:
    t = [0, 0, 0]
    for p in by:
        for x in by[p].get(c, {}).values():
            st = x.get("stats", {})
            for j, k in enumerate(("dfsbb_plunges", "dfsbb_wcut", "dfsbb_leaf")):
                t[j] += st.get(k, 0) or 0
    print(f"{c}: 潜り {t[0]}、上限で打ち切り {t[1]} ({t[1] / max(t[0], 1):.0%})、葉・刈り込みで終わり {t[2]}")

# FJSUB の統計 (fjg・fjs の回の合計): [置いた, 置かなかった (分数値が多い), 置かなかった (範囲の外), 差し替え, 部分問題の解, 部分問題で更新, 全体で更新, 部分問題の割合%]
t = [0] * 8
for p in by:
    for x in [v for c in ("fjg", "fjs") for v in by[p].get(c, {}).values()]:
        for i, v in enumerate(x.get("stats", {}).get("fjsub") or []):
            t[i] += v
print("FJSUB 合計: 置いた {0}、置かなかった (分数値が多い {1}・範囲の外 {2})、差し替え {3}、部分問題の解 {4}、暫定解の更新 部分問題 {5}・全体 {6}".format(*t))
