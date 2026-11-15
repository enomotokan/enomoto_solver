"""MPS を読んで解き、解のファイルを書くコマンドラインの入り口 (Mittelmann のベンチマークなど、外の人が回す用)。

既定の求解の経路は変えない: MPS は HiGHS (highspy) で読み、`benchmark_highs._build_our_model` でモデルを作って
`PyModel.solve` を呼ぶ (`scripts/run_miplib_benchmark.py --worker-ours` と同じ経路)。

使い方:
    python scripts/enomoto_mip_cli.py PROBLEM.mps[.gz] [--time-limit 秒] [--threads N] [--mip-rel-gap G]
        [--solution 解.sol] [--time-limit-scope total|solve] [--log]

- `--time-limit` (既定: なし): 時間の上限 [秒]。`--time-limit-scope total` (既定) ではこの入り口の起動からの壁時計で数え、
  MPS の読み込み・モデルの組み立てに使った分を引いた残りをソルバーに渡す (外から測る壁時計の時間に合わせる)。
  `solve` ではソルバーにそのまま渡す (`run_miplib_benchmark.py` と同じ。読み込みの時間は数えない)。
- `--threads` (既定: 使える CPU の数 + 1 の半分): ソルバーに渡すスレッドの数。
- `--mip-rel-gap` (既定 0): 相対ギャップ。(上界 - 下界) <= G * max(|上界|, 1) で最適として止める。HiGHS の既定は 1e-4。
- `--solution`: 暫定解があれば MIPLIB の解の形 (1 行目 `=obj= 目的値`、以下 `列の名前 値`) で書く。
- 終わりに 1 行ずつ `status`・`objective`・`best_bound`・`gap`・`nodes`・`time_read`・`time_solve`・`time_total` を出し、
  最後の行に同じ内容の JSON を出す。
- `--log`: ソルバーの経過 (前処理・根の LP・探索) を標準出力に出す。
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
import time
from pathlib import Path

T0 = time.perf_counter()


def main() -> int:
    ap = argparse.ArgumentParser(description="ENOMOTO-Solver: MPS を読んで (混合) 整数最適化問題・LP を解く")
    ap.add_argument("mps", help="MPS ファイル (.mps / .mps.gz。HiGHS が読める形)")
    ap.add_argument("--time-limit", type=float, default=None, help="時間の上限 [秒] (既定: なし)")
    ap.add_argument("--time-limit-scope", choices=["total", "solve"], default="total",
                    help="total: 起動からの壁時計で数える (既定)。solve: 求解だけに渡す")
    ap.add_argument("--threads", type=int, default=None, help="スレッドの数 (既定: CPU の数 + 1 の半分)")
    ap.add_argument("--mip-rel-gap", type=float, default=None, help="相対ギャップ (既定 0。HiGHS の既定は 1e-4)")
    ap.add_argument("--solution", default=None, help="解を書くファイル")
    ap.add_argument("--log", action="store_true", help="ソルバーの経過を出す")
    a = ap.parse_args()

    if a.threads is not None and a.threads >= 1:
        # ベンチマークの計測 (run_miplib_benchmark.py の run_worker) と同じく、根の並列の手順のスレッド数もそろえる
        os.environ.setdefault("ENOMOTO_MIP_THREADS", str(a.threads))

    from enomoto_solver import benchmark_highs as bh

    _, lp = bh._load_lp(Path(a.mps))
    names = list(lp.col_names_) if len(lp.col_names_) == lp.num_col_ else [f"C{j}" for j in range(lp.num_col_)]
    m, _, _ = bh._build_our_model(lp)
    t_read = time.perf_counter() - T0

    tl = a.time_limit
    if tl is not None and a.time_limit_scope == "total":
        tl = max(tl - t_read, 0.0)
    t1 = time.perf_counter()
    r = m.solve(None, False, tl, a.mip_rel_gap, None, a.threads, a.log)
    t_solve = time.perf_counter() - t1

    x = r.get("x")
    if a.solution and x is not None:
        with open(a.solution, "w") as f:
            f.write(f"=obj= {r['objective']!r}\n")
            for n, v in zip(names, x):
                f.write(f"{n} {float(v)!r}\n")

    obj, bnd = r.get("objective"), r.get("best_bound")
    gap = None
    if obj is not None and bnd is not None and math.isfinite(obj) and math.isfinite(bnd):
        gap = abs(obj - bnd) / max(abs(obj), 1e-9) if obj != bnd else 0.0
    out = {
        "status": r["status"],
        "objective": obj,
        "best_bound": bnd,
        "gap": gap,
        "nodes": r.get("nodes"),
        "time_read": t_read,
        "time_solve": t_solve,
        "time_total": time.perf_counter() - T0,
    }
    for k, v in out.items():
        print(f"{k:12s} {v}")
    print(json.dumps(out))
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
