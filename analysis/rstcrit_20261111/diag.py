"""RSTCRIT の再スタートごとの診断 (analysis/rstcrit_20261111.md): python diag.py 設定 ファイル...
再スタート (go) ごとに: 新しい固定の割合 (自由な整数列に対する) / 非零の縮み (nnznew/nnzfree) / 再スタートの前処理が足した固定の割合 (次の根の自由な整数列から) / 再スタートの手間 (go から次の根の判定 check (なければ木の始め) までの秒) / 下界の伸び (go の時の LP 値 -> 次の check の LP 値、
暫定解とのギャップに対する割合) を出し、非零の縮みの段ごとに数・手間の和・下界の伸びの中央値をまとめる。"""
import json, sys, math
from collections import defaultdict
cfg = sys.argv[1]
recs = []
for f in sys.argv[2:]:
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == cfg:
            recs.append(r)
out = []
for r in recs:
    d = r.get("diag") or []
    roots = {int(x["restarts"]): x for x in d if x["tag"] == "root"}
    for i, x in enumerate(d):
        if x["tag"] != "go":
            continue
        k = int(x["restarts"])
        nxt = next((y for y in d[i + 1:] if y["tag"] in ("check", "tree") and int(y["restarts"]) == k + 1), None)
        r0, r1 = roots.get(k), roots.get(k + 1)
        cost = (nxt["outer"] - x["outer"]) if nxt else float("nan")
        lp0 = x["lp"]
        lp1 = nxt.get("lp", nxt.get("lb")) if nxt else float("nan")
        inc = x["inc"]
        gap = inc - lp0 if math.isfinite(inc) else float("nan")
        gain = (lp1 - lp0) / gap if gap and gap > 1e-9 and math.isfinite(lp1) else float("nan")
        # 再スタートの前処理が新しい固定のほかに固定した整数列 (自由な整数列に対する割合)
        nnz_post = ((x["free0"] - x["newly"] - r1["free0"]) / max(x["free0"], 1)) if r1 else float("nan")
        out.append(dict(problem=r["problem"], rep=r["rep"], k=k + 1, frac=x["newly"] / max(x["free0"], 1), shrink=x["nnznew"] / max(x["nnzfree"], 1),
                        post=nnz_post, cost=cost, root=x["solve"], gain=gain, t=x["outer"], rows=x["m"], nnz=x["nnz"]))
print(f"{'問題':28s} 種 回 固定%  非零縮%  前処理追加% 手間s 根s  下界の伸び(ギャップ比)")
for o in out:
    print(f"{o['problem']:28s} {o['rep']} {o['k']}  {100*o['frac']:5.1f} {100*o['shrink']:6.1f} {100*o['post']:8.1f} {o['cost']:6.2f} {o['root']:5.2f} {o['gain']:7.3f}")
bins = [(0, 0.02), (0.02, 0.05), (0.05, 0.1), (0.1, 0.2), (0.2, 1.01)]
print("\n非零の縮みの段ごと: 数 / 手間の和 (秒) / 下界の伸びの中央値 / 伸びが 1% 未満の数")
for a, b in bins:
    xs = [o for o in out if a <= o["shrink"] < b]
    g = sorted(o["gain"] for o in xs if math.isfinite(o["gain"]))
    med = g[len(g) // 2] if g else float("nan")
    small = sum(1 for v in g if v < 0.01)
    print(f"  [{100*a:4.0f}%, {100*b:4.0f}%): {len(xs):3d} 回 / {sum(o['cost'] for o in xs if math.isfinite(o['cost'])):7.1f} 秒 / {med:.3f} / {small}")
