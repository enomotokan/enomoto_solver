"""`scripts/mf_bench/run.py` の結果 (JSON Lines) を、問題ごと・版ごとの中央値の表 (Markdown) にする。

各版の内点法の数値分解の累計 (`ipm_factor`) と全体の時間 (`time`) の中央値、基準の版に対する比、最大常駐メモリ、
比の幾何平均を出す。使い方: python scripts/mf_bench/report.py ab.jsonl [--base base] [--md OUT.md]
"""
from __future__ import annotations

import argparse
import json
import math
import statistics as st
from collections import defaultdict
from pathlib import Path


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("jsonl", type=Path)
    ap.add_argument("--base", default="base")
    ap.add_argument("--md", type=Path)
    args = ap.parse_args()
    d = defaultdict(list)
    probs, variants = [], []
    for line in args.jsonl.read_text().splitlines():
        r = json.loads(line)
        d[(r["problem"], r["variant"])].append(r)
        if r["problem"] not in probs:
            probs.append(r["problem"])
        if r["variant"] not in variants:
            variants.append(r["variant"])
    med = lambda rs, k: st.median([r[k] for r in rs if r.get(k) is not None]) if any(r.get(k) is not None for r in rs) else float("nan")
    out = []
    for key, title in [("ipm_factor", "内点法の数値分解の累計 (秒、中央値)"), ("time", "全体の時間 (秒、中央値)")]:
        out.append(f"\n### {title}\n")
        out.append("| 問題 | " + " | ".join(variants) + " |")
        out.append("|---|" + "---|" * len(variants))
        logs = defaultdict(list)
        for p in probs:
            b = med(d[(p, args.base)], key)
            cells = []
            for v in variants:
                m = med(d[(p, v)], key)
                ok = all(r["status"] == "optimal" for r in d[(p, v)])
                if v == args.base or b != b:
                    cells.append(f"{m:.1f}" + ("" if ok else " (!)"))
                else:
                    cells.append(f"{m:.1f} ({m / b:.3f})" + ("" if ok else " (!)"))
                    if m == m and b == b:
                        logs[v].append(math.log(m / b))
            out.append(f"| {p} | " + " | ".join(cells) + " |")
        out.append("| 幾何平均の比 (基準が解けた問題) | - | " + " | ".join(f"{math.exp(sum(logs[v]) / len(logs[v])):.3f}" for v in variants if v != args.base) + " |")
    out.append("\n### 最大常駐メモリ (MB、各版の最大)\n")
    out.append("| 問題 | " + " | ".join(variants) + " |")
    out.append("|---|" + "---|" * len(variants))
    for p in probs:
        b = max(r.get("maxrss_mb", 0) for r in d[(p, args.base)])
        cells = []
        for v in variants:
            m = max(r.get("maxrss_mb", 0) for r in d[(p, v)])
            cells.append(f"{m:.0f}" if v == args.base or not b else f"{m:.0f} ({m / b:.2f})")
        out.append(f"| {p} | " + " | ".join(cells) + " |")
    text = "\n".join(out)
    print(text)
    if args.md:
        args.md.write_text(text + "\n")


if __name__ == "__main__":
    main()
