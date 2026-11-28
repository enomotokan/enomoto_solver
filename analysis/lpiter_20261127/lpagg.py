"""lpbench.py の結果 (lpbench_*.jsonl) を 10 節の表にまとめる。

python analysis/lpiter_20261127/lpagg.py [simplex|auto4]
- 問題・設定ごとに、解けた回の中央値 (時間・反復数・FTRAN・BTRAN の ms) をとる。解けなかったら 600 秒として扱う。
- 平均は幾何平均。全体の時間はずらし幾何平均 (ずらし 1 秒) も出す。
- 単体法の時間・反復あたりの比は、計時された単体法 (PROF_PHASES_EXT) が全体の時間の半分以上を占める問題だけで出す
  (rail4284 は 73 秒のうち計時された単体法が 22 ms で、残りは別の段階)。
- 反復数が同じ問題 (def と比べる設定とで単体法の反復数が一致) と、違う問題を分ける。
"""
import json, math, statistics, sys
from pathlib import Path

D = Path(__file__).resolve().parent
TL = 600.0
OK = ("optimal", "infeasible", "unbounded")
SETS = {"netlib": ["netlib"], "kennington": ["kennington"], "mittelmann": ["mittelmann_s0", "mittelmann_s1"]}
CFGS = ["def", "rtib", "rtibo", "highs"]


def load(mode):
    out = {}
    for s, tags in SETS.items():
        rows = {}
        for t in tags:
            base, _, sh = t.partition("_")
            f = D / (f"lpbench_{base}_{mode}" + (f"_{sh}" if sh else "") + ".jsonl")
            if not f.exists():
                continue
            for l in f.read_text().splitlines():
                r = json.loads(l)
                rows.setdefault(r["problem"], {}).setdefault(r["cfg"], []).append(r["result"])
        out[s] = rows
    return out


def summ(rs):
    ok = [r for r in rs if r.get("status") in OK]
    if not ok:
        return {"ok": False, "time": TL, "status": rs[0].get("status") if rs else None}
    med = lambda f: statistics.median(f(r) for r in ok)
    d = {"ok": True, "time": med(lambda r: r["time"]), "n": len(ok)}
    if "prof" in ok[0]:
        d["iters"] = ok[0]["prof"]["iters"]
        d["iters_all"] = sorted({r["prof"]["iters"] for r in ok})
        d["simplex"] = med(lambda r: r["prof"]["wall_ms"]) / 1e3
        d["ftran"] = med(lambda r: r["prof"].get("ftran", 0.0)) / 1e3
        d["btran"] = med(lambda r: r["prof"].get("btran(rho_p)", 0.0)) / 1e3
    else:
        d["iters"] = ok[0].get("iters")
    return d


def gm(xs):
    xs = [x for x in xs if x > 0]
    return math.exp(sum(map(math.log, xs)) / len(xs)) if xs else float("nan")


def sgm(xs, s=1.0):
    return math.exp(sum(math.log(x + s) for x in xs) / len(xs)) - s


def f3(x):
    return "—" if x is None or (isinstance(x, float) and math.isnan(x)) else f"{x:.3f}"


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "simplex"
    data = load(mode)
    S = {s: {p: {c: summ(v) for c, v in cs.items()} for p, cs in rows.items()} for s, rows in data.items()}
    print(f"# mode = {mode}\n")
    print("## 全体 (解けた数、時間のずらし幾何平均 [秒、解けなければ 600 秒])\n")
    print("| 集合 | 問題数 | " + " | ".join(f"{c} 解けた" for c in CFGS) + " | " + " | ".join(f"{c} SGM" for c in CFGS) + " |")
    print("|---|---" + "|---" * (2 * len(CFGS)) + "|")
    for s, P in S.items():
        ps = [p for p in P if all(c in P[p] for c in CFGS)]
        if not ps:
            continue
        print(f"| {s} | {len(ps)} | " + " | ".join(str(sum(P[p][c]["ok"] for p in ps)) for c in CFGS) + " | "
              + " | ".join(f"{sgm([P[p][c]['time'] for p in ps]):.3f}" for c in CFGS) + " |")
    print()
    if mode != "simplex":
        print("## 問題ごとの時間 (秒)\n")
        for s, P in S.items():
            for p in sorted(P):
                print(f"| {s} | {p} | " + " | ".join(
                    (f"{P[p][c]['time']:.2f}" if P[p][c]["ok"] else f"×{P[p][c]['status']}") if c in P[p] else "—" for c in CFGS) + " |")
        return
    for c in ("rtib", "rtibo"):
        for base in ("def",) if c == "rtib" else ("def", "rtib"):
            print(f"## {c} / {base}\n")
            print("| 集合 | 区分 | 問題数 | 時間比 | 単体法の時間比 | 秒/反復の比 | FTRAN/反復の比 | BTRAN/反復の比 |")
            print("|---|---|---|---|---|---|---|---|")
            for s, P in S.items():
                both = [p for p in P if c in P[p] and base in P[p] and P[p][c]["ok"] and P[p][base]["ok"]]
                same = [p for p in both if P[p][c]["iters"] == P[p][base]["iters"]]
                diff = [p for p in both if P[p][c]["iters"] != P[p][base]["iters"]]
                for lab, ps in (("反復同じ", same), ("反復同じ・0.1 秒以上", [p for p in same if P[p][base]["time"] >= 0.1]),
                                ("反復違う", diff)):
                    if not ps:
                        continue
                    r = lambda k, per=False: gm([(P[p][c][k] / (P[p][c]["iters"] if per else 1)) /
                                                 (P[p][base][k] / (P[p][base]["iters"] if per else 1))
                                                 for p in ps if P[p][base][k] > 0 and P[p][c][k] > 0 and P[p][c]["iters"]
                                                 and (k == "time" or P[p][base]["simplex"] >= 0.5 * P[p][base]["time"])])
                    print(f"| {s} | {lab} | {len(ps)} | {f3(r('time'))} | {f3(r('simplex'))} | {f3(r('simplex', True))} | "
                          f"{f3(r('ftran', True))} | {f3(r('btran', True))} |")
                only = [(p, P[p][c]["ok"], P[p][base]["ok"]) for p in P if c in P[p] and base in P[p] and P[p][c]["ok"] != P[p][base]["ok"]]
                for p, a, b in only:
                    print(f"| {s} | 片方だけ解けた | {p} | {c}: {'解けた' if a else '×'} / {base}: {'解けた' if b else '×'} | | | | |")
            print()
    print("## 反復数が def と違う問題 (反復数、時間 [秒])\n")
    print("| 集合 | 問題 | def 反復 | rtib 反復 | rtibo 反復 | def 時間 | rtib 時間 | rtibo 時間 |")
    print("|---|---|---|---|---|---|---|---|")
    for s, P in S.items():
        for p in sorted(P):
            x = P[p]
            if not all(c in x and x[c]["ok"] for c in ("def", "rtib", "rtibo")):
                continue
            if x["def"]["iters"] == x["rtib"]["iters"] == x["rtibo"]["iters"]:
                continue
            print(f"| {s} | {p} | {x['def']['iters']} | {x['rtib']['iters']} | {x['rtibo']['iters']} | "
                  f"{x['def']['time']:.3f} | {x['rtib']['time']:.3f} | {x['rtibo']['time']:.3f} |")
    print()
    print("## 回ごとに反復数がぶれた問題\n")
    for s, P in S.items():
        for p in sorted(P):
            for c in ("def", "rtib", "rtibo"):
                if c in P[p] and P[p][c]["ok"] and len(P[p][c]["iters_all"]) > 1:
                    print(f"- {s} {p} {c}: {P[p][c]['iters_all']}")
    print()
    print("## HiGHS との比 (当方 def / HiGHS、両方解けた問題)\n")
    print("| 集合 | 問題数 | 時間比 (幾何平均) | 反復数比 | 秒/反復の比 (当方は単体法の時間) |")
    print("|---|---|---|---|---|")
    for s, P in S.items():
        ps = [p for p in P if "def" in P[p] and "highs" in P[p] and P[p]["def"]["ok"] and P[p]["highs"]["ok"]
              and P[p]["highs"]["iters"]]
        for lab, qs in (("全部", ps), ("0.1 秒以上", [p for p in ps if P[p]["highs"]["time"] >= 0.1])):
            if not qs:
                continue
            print(f"| {s} {lab} | {len(qs)} | {gm([P[p]['def']['time'] / P[p]['highs']['time'] for p in qs]):.3f} | "
                  f"{gm([P[p]['def']['iters'] / P[p]['highs']['iters'] for p in qs]):.3f} | "
                  f"{gm([(P[p]['def']['simplex'] / P[p]['def']['iters']) / (P[p]['highs']['time'] / P[p]['highs']['iters']) for p in qs]):.3f} |")
    print()
    print("## Mittelmann の問題ごと (時間 [秒] / 反復数)\n")
    print("| 問題 | " + " | ".join(CFGS) + " |")
    print("|---|" + "---|" * len(CFGS))
    P = S["mittelmann"]
    for p in sorted(P):
        print(f"| {p} | " + " | ".join(
            (f"{P[p][c]['time']:.1f} / {P[p][c]['iters']}" if P[p][c]["ok"] else f"× ({P[p][c]['status']})") if c in P[p] else "—"
            for c in CFGS) + " |")


if __name__ == "__main__":
    main()
