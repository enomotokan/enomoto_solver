"""Netlib の実行不能問題から非有界問題の集合を作る。

各実行不能問題 `min c^T x, row_lower <= Ax <= row_upper, col_lower <= x <= col_upper` について、
目的関数を 0 にした問題の双対をとる。制約を `G x >= h`(有限な下限側・上限側と変数の有限な上下限を
それぞれ 1 本の `>=` 行にしたもの、双対変数 `y >= 0`)と `E x = b`(等式行、双対変数 `z` は自由)に
分けると、双対は

    max h^T y + b^T z   s.t.  G^T y + E^T z = 0,  y >= 0

で、`y = z = 0` で必ず実行可能。主問題が実行不能なので(Farkas の補題により)双対は非有界になる。
ここでは `min -(h^T y + b^T z)` として書き出す。

使い方(リポジトリのルートで):
    python scripts/make_dual_unbounded.py [--src .netlib_infeas_cache] [--dst .netlib_unbd_cache]

`--src/problems.txt` と `--src/mps/<名前>.mps` を読み、`--dst/mps/<名前>_dual.mps` と
`--dst/problems.txt` を書く。`highspy` が必要。

cplex2 は除外する。Netlib の解説どおり「ほぼ実行可能」な問題で、実行不能性の証明書の大きさが
許容誤差程度しかなく、双対は HiGHS でも最適(最適値 0)と判定されて非有界問題にならないため。
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path

import highspy

# 双対が数値的に非有界にならないので除外する問題(docstring 参照)。
EXCLUDE = {"cplex2"}


def dual_of_feasibility_problem(lp) -> highspy.HighsLp:
    """`lp` の目的関数を 0 にした問題の双対(上の形)を `HighsLp` で返す。"""
    n, m = lp.num_col_, lp.num_row_
    col_lower = list(lp.col_lower_)
    col_upper = list(lp.col_upper_)
    row_lower = list(lp.row_lower_)
    row_upper = list(lp.row_upper_)
    am = lp.a_matrix_
    start, index, value = list(am.start_), list(am.index_), list(am.value_)
    # 行優先に並べ替える(双対の列 = 主問題の行)。
    rows: list[list[tuple[int, float]]] = [[] for _ in range(m)]
    if "kColwise" in str(am.format_):
        for j in range(n):
            for k in range(start[j], start[j + 1]):
                if value[k] != 0.0:
                    rows[index[k]].append((j, value[k]))
    else:
        for i in range(m):
            for k in range(start[i], start[i + 1]):
                if value[k] != 0.0:
                    rows[i].append((index[k], value[k]))

    # 双対の列: (主問題の変数ごとの係数リスト, 目的関数係数 (max 側), 自由か)
    dual_cols: list[tuple[list[tuple[int, float]], float, bool]] = []
    for i in range(m):
        lo, hi = row_lower[i], row_upper[i]
        if math.isfinite(lo) and math.isfinite(hi) and lo == hi:
            dual_cols.append((rows[i], lo, True))  # 等式: z 自由
            continue
        if math.isfinite(lo):
            dual_cols.append((rows[i], lo, False))  # a x >= lo
        if math.isfinite(hi):
            dual_cols.append(([(j, -v) for j, v in rows[i]], -hi, False))  # -a x >= -hi
    for j in range(n):
        lo, hi = col_lower[j], col_upper[j]
        if math.isfinite(lo) and math.isfinite(hi) and lo == hi:
            dual_cols.append(([(j, 1.0)], lo, True))
            continue
        if math.isfinite(lo):
            dual_cols.append(([(j, 1.0)], lo, False))  # x_j >= lo
        if math.isfinite(hi):
            dual_cols.append(([(j, -1.0)], -hi, False))  # -x_j >= -hi

    d = highspy.HighsLp()
    d.num_col_ = len(dual_cols)
    d.num_row_ = n
    d.col_cost_ = [-h for _, h, _ in dual_cols]  # min -(h^T y + b^T z)
    d.col_lower_ = [-highspy.kHighsInf if free else 0.0 for _, _, free in dual_cols]
    d.col_upper_ = [highspy.kHighsInf] * len(dual_cols)
    d.row_lower_ = [0.0] * n
    d.row_upper_ = [0.0] * n
    s, idx, val = [0], [], []
    for entries, _, _ in dual_cols:
        for j, v in entries:
            idx.append(j)
            val.append(v)
        s.append(len(idx))
    d.a_matrix_.format_ = highspy.MatrixFormat.kColwise
    d.a_matrix_.start_ = s
    d.a_matrix_.index_ = idx
    d.a_matrix_.value_ = val
    d.a_matrix_.num_col_ = d.num_col_
    d.a_matrix_.num_row_ = n
    return d


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--src", type=Path, default=Path(".netlib_infeas_cache"))
    ap.add_argument("--dst", type=Path, default=Path(".netlib_unbd_cache"))
    args = ap.parse_args()
    (args.dst / "mps").mkdir(parents=True, exist_ok=True)
    names = []
    for name in (args.src / "problems.txt").read_text().split():
        if name in EXCLUDE:
            continue
        h = highspy.Highs()
        h.setOptionValue("output_flag", False)
        status = h.readModel(str(args.src / "mps" / f"{name}.mps"))
        if "kOk" not in str(status) and "kWarning" not in str(status):
            print(f"{name}: read failed ({status})")
            continue
        dual = dual_of_feasibility_problem(h.getLp())
        out = highspy.Highs()
        out.setOptionValue("output_flag", False)
        out.passModel(dual)
        dname = f"{name}_dual"
        out.writeModel(str(args.dst / "mps" / f"{dname}.mps"))
        names.append(dname)
        print(f"{dname}: {dual.num_row_} rows, {dual.num_col_} cols")
    (args.dst / "problems.txt").write_bytes(("\n".join(names) + "\n").encode())  # OS によらず LF


if __name__ == "__main__":
    main()
