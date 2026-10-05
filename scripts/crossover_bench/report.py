"""run.py の結果 (JSON Lines) から比較表 (Markdown) を作る。

    python scripts/crossover_bench/report.py benchmarks/crossover/results.jsonl > benchmarks/crossover/summary.md
"""
from __future__ import annotations

import json
import math
import sys
from collections import defaultdict

LABEL = {"slope_intercept": "傾き・切片双対二段解法", "ipm_crossover": "内点法 + クロスオーバー"}
SET_LABEL = {"netlib": "Netlib (有限最適解あり 93 問)", "kennington": "Kennington (前処理後 5000 行以上)",
             "mittelmann": "Mittelmann LPopt (前処理後 5000 行以上)"}
PHASES = [("ipm_end", "内点法"), ("primal_push_end", "主の押し出し"), ("dual_push_end", "双対の押し出し"),
          ("basis_end", "基底の選択"), ("cleanup_end", "仕上げ (単体法)")]


def sgm(ts, shift=10.0):
    return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift


def gm(ts):
    return math.exp(sum(math.log(max(t, 1e-6)) for t in ts) / len(ts))


def main() -> None:
    path = sys.argv[1]
    limit = float(sys.argv[2]) if len(sys.argv) > 2 else 600.0
    recs = [json.loads(l) for l in open(path)]
    by = defaultdict(dict)
    for r in recs:
        by[(r["set"], r["problem"])][r["method"]] = r
    sets = [s for s in SET_LABEL if any(k[0] == s for k in by)]
    methods = ["slope_intercept", "ipm_crossover"]
    out = []
    out.append("| 問題集合 | 問題数 | " + " | ".join(f"{LABEL[m]}: 解けた数 / ずらした幾何平均 (秒) / 幾何平均 (秒)" for m in methods) + " | 時間比 (内点法+XO / 二段解法、幾何平均) |")
    out.append("|---|---|" + "---|" * len(methods) + "---|")
    for s in sets + ["all"]:
        keys = [k for k in by if (s == "all" or k[0] == s) and all(m in by[k] for m in methods)]
        if not keys:
            continue
        cells = []
        tm = {}
        for m in methods:
            ts = [by[k][m]["time"] if by[k][m]["solved"] else limit for k in keys]
            tm[m] = ts
            cells.append(f"{sum(by[k][m]['solved'] for k in keys)} / {sgm(ts):.3g} / {gm(ts):.3g}")
        ratio = gm([a / b for a, b in zip(tm["ipm_crossover"], tm["slope_intercept"])])
        out.append(f"| {SET_LABEL.get(s, '全体')} | {len(keys)} | " + " | ".join(cells) + f" | {ratio:.2f} |")
    out.append("")
    # 段階別
    out.append("### 内点法 + クロスオーバーの段階別の時間 (クロスオーバーまで進んだ問題の合計、秒)")
    out.append("")
    out.append("| 問題集合 | 問題数 | 前処理 | " + " | ".join(p[1] for p in PHASES) + " | 単体法へ戻した問題 |")
    out.append("|---|---|---|" + "---|" * len(PHASES) + "---|")
    for s in sets:
        tot = defaultdict(float)
        cnt = 0
        fb = []
        for k in by:
            if k[0] != s or "ipm_crossover" not in by[k]:
                continue
            ev = dict((a, b) for a, b in (by[k]["ipm_crossover"].get("events") or []))
            if "crossover_fallback" in ev:
                fb.append(k[1])
                continue
            if "cleanup_end" not in ev:
                continue
            cnt += 1
            prev = ev.get("presolve_end", 0.0)
            tot["presolve"] += prev
            for key, _ in PHASES:
                tot[key] += ev[key] - prev
                prev = ev[key]
        out.append(f"| {SET_LABEL[s]} | {cnt} | {tot['presolve']:.2f} | " + " | ".join(f"{tot[k]:.2f}" for k, _ in PHASES)
                   + f" | {len(fb)} ({', '.join(fb)}) |")
    out.append("")
    # 問題ごと
    for s in sets:
        out.append(f"### {SET_LABEL[s]}")
        out.append("")
        out.append("| 問題 | 二段解法 (秒) | 内点法+XO (秒) | 比 | 内点法 | 押し出し (主+双対) | 基底選択 | 仕上げ | 備考 |")
        out.append("|---|---|---|---|---|---|---|---|---|")
        for k in sorted(k for k in by if k[0] == s):
            a, b = by[k].get("slope_intercept"), by[k].get("ipm_crossover")

            def cell(r):
                if r is None:
                    return "-"
                return f"{r['time']:.3f}" if r["solved"] else f"{r['status']}"
            ratio = f"{b['time'] / a['time']:.2f}" if a and b and a["solved"] and b["solved"] else "-"
            ev = dict((x, y) for x, y in ((b or {}).get("events") or []))
            note = ""
            ph = ["-"] * 4
            if "crossover_fallback" in ev:
                note = f"内点法が収束せず二段解法で解き直し ({ev['crossover_fallback']:.2f} 秒で切替)"
                ph[0] = f"{ev.get('ipm_end', 0) - ev.get('presolve_end', 0):.2f}"
            elif "cleanup_end" in ev:
                ph = [f"{ev['ipm_end'] - ev['presolve_end']:.2f}", f"{ev['dual_push_end'] - ev['ipm_end']:.2f}",
                      f"{ev['basis_end'] - ev['dual_push_end']:.2f}", f"{ev['cleanup_end'] - ev['basis_end']:.2f}"]
            out.append(f"| {k[1]} | {cell(a)} | {cell(b)} | {ratio} | " + " | ".join(ph) + f" | {note} |")
        out.append("")
    print("\n".join(out))


if __name__ == "__main__":
    main()
