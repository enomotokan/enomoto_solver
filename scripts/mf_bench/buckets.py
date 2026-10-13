"""`scripts/mf_bench/run.py --mf-prof --save-stderr DIR` の標準エラーから、マルチフロンタル法の前線の大きさ別の内訳
(`MFBUCKET` の行、分解ごと) を問題ごとに合計して表にする。

区分: f < 32, < 128, < 512, < 2048, それ以上 (逐次)、並列に計算した大きな前線 (big)。時間はスレッドの和
(big の前線は 1 つの前線を全スレッドで計算するので壁時計の時間)。GFLOPS は演算量 / 時間。

使い方: python scripts/mf_bench/buckets.py DIR [--md OUT.md]
"""
from __future__ import annotations

import argparse
import re
from collections import defaultdict
from pathlib import Path

NAMES = ["f<32", "f<128", "f<512", "f<2048", "f>=2048", "big(par)"]
PHASES = ["asm+ea", "chol", "trsm", "syrk"]


def load(path: Path) -> dict:
    agg = defaultdict(lambda: [0, [0.0] * 4, [0.0] * 3])
    for line in path.read_text().splitlines():
        m = re.match(r"MFBUCKET b=(\d+) n=(\d+) t=\[([^\]]*)\] flops=\[([^\]]*)\]", line)
        if not m:
            continue
        a = agg[int(m.group(1))]
        a[0] += int(m.group(2))
        for i, v in enumerate(m.group(3).split(",")):
            a[1][i] += float(v)
        for i, v in enumerate(m.group(4).split(",")):
            a[2][i] += float(v)
    return agg


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("dir", type=Path)
    ap.add_argument("--md", type=Path)
    args = ap.parse_args()
    out = []
    for f in sorted(args.dir.glob("*.txt")):
        agg = load(f)
        if not agg:
            continue
        tot = sum(sum(a[1]) for a in agg.values())
        out.append(f"\n### {f.stem}  (合計 {tot:.1f} 秒)\n")
        out.append("| 区分 | 節点 (分解の回数の和) | 時間の割合 | asm+ea 秒 | chol 秒 (GFLOPS) | trsm 秒 (GFLOPS) | syrk 秒 (GFLOPS) |")
        out.append("|---|---|---|---|---|---|---|")
        for b in sorted(agg):
            n, t, fl = agg[b]
            g = [fl[i] / t[i + 1] * 1e-9 if t[i + 1] > 0 else 0.0 for i in range(3)]
            out.append(f"| {NAMES[b]} | {n} | {100 * sum(t) / tot:.0f}% | {t[0]:.2f} | {t[1]:.2f} ({g[0]:.1f}) | {t[2]:.2f} ({g[1]:.1f}) | {t[3]:.2f} ({g[2]:.1f}) |")
    text = "\n".join(out)
    print(text)
    if args.md:
        args.md.write_text(text + "\n")


if __name__ == "__main__":
    main()
