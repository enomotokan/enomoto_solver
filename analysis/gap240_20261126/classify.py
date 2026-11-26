"""(a)・(b) の問題を原因の型に分け、型ごとの SGM への効きを見積もる: python classify.py > classify.txt
型の付け方の根拠は README 3 節 (根の下界は losses_20261123 6.4 節の rootcmp、ほかは logs.txt と過去の記録)。
効き = その型の問題で、当方の時間を相手の時間まで縮めたときの 186 問の SGM (shift 10) の比 (table.py の eff と同じ)。
既定 (def) のほかに、c2c (combo2 の第一候補) を入れたあとに残る差も出す。"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
TL, SH, N = 600.0, 10.0, 186
rows = {d["problem"]: d for d in json.loads((D / "gap.json").read_text())}

# 問題 → (相手, 主な型, 従の型)。相手 scip は当方 t1 と、highs は当方 t4 と比べる (公平な対)
TYPE = {
    # SCIP (逐次) に負ける: 当方 1 スレッド
    "30n20b8": ("scip", "解 (1 スレッド)", "ノード数・根の下界"),
    "CMS750_4": ("scip", "解 (1 スレッド)", ""),
    "bnatt400": ("scip", "解 (1 スレッド)", "ノードあたり"),
    "neos-1171448": ("scip", "解 (1 スレッド)", ""),
    "dano3_5": ("scip", "ノード数 (分枝)", "解"),
    "mcsched": ("scip", "ノード数 (分枝)", ""),
    "swath1": ("scip", "ノード数 (分枝)", ""),
    "istanbul-no-cutoff": ("scip", "根の下界 (前処理・カット)", ""),
    "rocI-4-11": ("scip", "ノードあたり (1 スレッド)", ""),
    "fhnw-binpack4-4": ("scip", "ノードあたり", ""),
    "tbfp-network": ("scip", "根の手間 (1 スレッド)", ""),
    # HiGHS (4 スレッド) に負ける: 当方 4 スレッド
    "csched008": ("highs", "木を閉じる (最後の 1 段)", "根の再スタート"),
    "sct2": ("highs", "木を閉じる (最後の 1 段)", "根の再スタート"),
    "uccase12": ("highs", "根の再スタート・縮小", "ノードあたり"),
    "satellites2-40": ("highs", "根の再スタート・縮小", "ノードあたり・解"),
    "satellites2-60-fs": ("highs", "根の再スタート・縮小", "ノードあたり・解"),
    "neos-873061": ("highs", "根の下界 (前処理・カット)", "根の再スタート・根の手間"),
    "net12": ("highs", "根の下界 (前処理・カット)", "ノードあたり"),
    "neos859080": ("highs", "4 スレッドだけ (根の LP の競争)", ""),
}


def ok(x): return x is not None and x["status"] in ("optimal", "infeasible")
def T(x): return x["time"] if ok(x) else TL
def eff(e, o): return math.exp((math.log(min(e, o) + SH) - math.log(e + SH)) / N)


by = {}
print("| 問題 | 相手 | 主な型 | 従の型 | 当方 既定 | 当方 c2c | 相手 | 効き (既定) | 効き (c2c を入れた後) |")
print("|---|---|---|---|---:|---:|---:|---:|---:|")
for p, (opp, main, sub) in TYPE.items():
    d = rows[p]; th = "t1" if opp == "scip" else "t4"
    e, c, o = T(d[f"def_{th}"]), T(d[f"c2c_{th}"]), T(d[opp])
    ed, ec = eff(e, o), eff(c, o)
    by.setdefault(main, []).append((p, ed, ec, th))
    print(f"| {p} | {opp} ({th}) | {main} | {sub} | {e:.0f} | {c:.0f} | {o:.0f} | {ed:.4f} | {ec:.4f} |")

print("\n== 型ごとの見積り (主な型で数える。効きは 186 問の SGM の比、1 より小さいほど SGM が縮む)\n")
print("| 型 | 問題の数 | 問題 | 効き t1 (既定) | 効き t4 (既定) | 効き t1 (c2c の後) | 効き t4 (c2c の後) |")
print("|---|---:|---|---:|---:|---:|---:|")
order = sorted(by.items(), key=lambda kv: math.prod(x[1] for x in kv[1]))
for k, v in order:
    def pr(th, i): return math.prod(x[i] for x in v if x[3] == th)
    print(f"| {k} | {len(v)} | {'、'.join(x[0] for x in v)} | {pr('t1', 1):.4f} | {pr('t4', 1):.4f} | {pr('t1', 2):.4f} | {pr('t4', 2):.4f} |")
allv = [x for v in by.values() for x in v]
print(f"| 計 | {len(allv)} | | {math.prod(x[1] for x in allv if x[3] == 't1'):.4f} | {math.prod(x[1] for x in allv if x[3] == 't4'):.4f} | "
      f"{math.prod(x[2] for x in allv if x[3] == 't1'):.4f} | {math.prod(x[2] for x in allv if x[3] == 't4'):.4f} |")
