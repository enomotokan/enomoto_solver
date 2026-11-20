"""このクレートの MPS 読み込み (`PyModel.read_mps`) と、従来の HiGHS 経由の組み立て
(`benchmark_highs._build_our_model`) が同じモデルを作るかを、問題ごとに確かめる。

計測の台本を HiGHS 経由からこのクレートの読み込みに切り替えたとき (2026-11-20) に、過去の結果と
同じ問題を解いているかを確かめるためのもの。比べるのは、変数の下限・上限・整数性、目的 (係数・定数・向き)、
制約の行数と非零数。違いがあれば問題名と中身を出す。

使い方: python scripts/check_mps_reader.py .miplib_cache/*.mps [--out 結果.jsonl]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

from enomoto_solver import benchmark_highs as bh


def compare(path: Path) -> dict:
    t = time.perf_counter()
    _, lp = bh._load_lp(path)
    ref, _, _ = bh._build_our_model(lp)
    t_highs = time.perf_counter() - t
    t = time.perf_counter()
    ours, _, _, names = bh._read_our_model(path)
    t_ours = time.perf_counter() - t
    diffs: list[str] = []
    a, b = ref.variable_bounds(), ours.variable_bounds()
    if len(a[0]) != len(b[0]):
        diffs.append(f"variables {len(a[0])} vs {len(b[0])}")
    else:
        for k, what in enumerate(["lb", "ub", "integer"]):
            d = [j for j in range(len(a[k])) if a[k][j] != b[k][j]]
            if d:
                j = d[0]
                diffs.append(f"{what}: {len(d)} columns, e.g. {names[j]} {a[k][j]} vs {b[k][j]}")
    oa, ob = ref.objective_terms(), ours.objective_terms()
    if oa != ob:
        diffs.append(f"objective: {len(oa[0])} vs {len(ob[0])} terms, constant {oa[1]} vs {ob[1]}, {oa[2]} vs {ob[2]}")
    ra, rb = (ref.n_constraints(), ref.n_nonzeros()), (ours.n_constraints(), ours.n_nonzeros())
    if ra != rb:
        diffs.append(f"rows/nonzeros {ra} vs {rb}")
    return {"problem": path.name, "same": not diffs, "diffs": diffs, "t_highs": t_highs, "t_ours": t_ours}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("mps", nargs="+", type=Path)
    ap.add_argument("--out", type=Path, default=None)
    a = ap.parse_args()
    n_same = 0
    out = a.out.open("w") if a.out else None
    for p in sorted(a.mps):
        try:
            r = compare(p)
        except Exception as e:  # noqa: BLE001 - 読めない問題も一覧に残す
            r = {"problem": p.name, "same": False, "diffs": [f"error: {e}"]}
        n_same += r["same"]
        if not r["same"]:
            print(f"{r['problem']}: " + "; ".join(r["diffs"]), flush=True)
        if out:
            out.write(json.dumps(r) + "\n")
            out.flush()
    print(f"same {n_same} / {len(a.mps)}")
    sys.exit(0 if n_same == len(a.mps) else 1)


if __name__ == "__main__":
    main()
