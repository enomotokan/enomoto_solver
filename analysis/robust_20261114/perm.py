"""行・列を乱数で並べ替えた同値な MPS を作る: python perm.py 元.mps 種 出力.mps
(`import perm; perm.make(元, 種, 出力)` でも使う)。

HiGHS (highspy) で読み、行と列の順 (と各列の中の非零の順) を種 k の乱数で入れ替えた LP を `passModel` → `writeModel` で書く。
名前は一緒に並べ替える (名前で列を見分ける処理があっても同じ列を指す)。書いたあと読み直し、逆の並べ替えで元と
目的・境界・整数性・行の範囲・非零 (値まで完全一致)・目的の定数・向きが同じことを確かめる (違えば例外)。
並べ替えは名前以外の情報を変えないので最適値は不変 (計測でも全回の目的値が一致することを summ.py で確かめる)。"""
import os
import random
import sys
from pathlib import Path

import highspy
import numpy as np


def _read(path):
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    st = h.readModel(str(path))
    if "kError" in str(st):
        raise RuntimeError(f"readModel failed: {path} {st}")
    return h, h.getLp()


def _cols(lp):
    """列ごとの {行: 値} (列優先に直して)。"""
    a = lp.a_matrix_
    n, m = lp.num_col_, lp.num_row_
    start, index, value = np.array(a.start_), np.array(a.index_), np.array(a.value_)
    if "kRowwise" in str(a.format_):  # 行優先なら列優先に直す
        cols = [dict() for _ in range(n)]
        for i in range(m):
            for p in range(start[i], start[i + 1]):
                cols[index[p]][i] = value[p]
        return cols
    return [dict(zip(index[start[j]:start[j + 1]].tolist(), value[start[j]:start[j + 1]].tolist())) for j in range(n)]


def make(src, k, out):
    _, lp = _read(src)
    n, m = lp.num_col_, lp.num_row_
    rng = random.Random(1000003 * k + 17)
    cp = list(range(n)); rng.shuffle(cp)  # 新しい列 j は元の列 cp[j]
    rp = list(range(m)); rng.shuffle(rp)  # 新しい行 i は元の行 rp[i]
    rinv = [0] * m
    for i, r in enumerate(rp):
        rinv[r] = i
    cols = _cols(lp)
    nl = highspy.HighsLp()
    nl.num_col_, nl.num_row_ = n, m
    cost, cl, cu = list(lp.col_cost_), list(lp.col_lower_), list(lp.col_upper_)
    rl, ru = list(lp.row_lower_), list(lp.row_upper_)
    nl.col_cost_ = np.array([cost[c] for c in cp], dtype=float)
    nl.col_lower_ = np.array([cl[c] for c in cp], dtype=float)
    nl.col_upper_ = np.array([cu[c] for c in cp], dtype=float)
    nl.row_lower_ = np.array([rl[r] for r in rp], dtype=float)
    nl.row_upper_ = np.array([ru[r] for r in rp], dtype=float)
    integ = list(lp.integrality_)
    if integ:
        nl.integrality_ = [integ[c] for c in cp]
    nl.offset_ = lp.offset_
    nl.sense_ = lp.sense_
    if len(lp.col_names_) == n:
        nl.col_names_ = [lp.col_names_[c] for c in cp]
    if len(lp.row_names_) == m:
        nl.row_names_ = [lp.row_names_[r] for r in rp]
    starts, idx, val = [0], [], []
    for c in cp:
        ent = [(rinv[r], v) for r, v in cols[c].items()]
        rng.shuffle(ent)  # 列の中の非零の順も入れ替える (HiGHS が並べ直すかもしれないが害はない)
        for r, v in ent:
            idx.append(r); val.append(v)
        starts.append(len(idx))
    nl.a_matrix_.format_ = highspy.MatrixFormat.kColwise
    nl.a_matrix_.num_col_, nl.a_matrix_.num_row_ = n, m
    nl.a_matrix_.start_ = np.array(starts, dtype=np.int32)
    nl.a_matrix_.index_ = np.array(idx, dtype=np.int32)
    nl.a_matrix_.value_ = np.array(val, dtype=float)
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    st = h.passModel(nl)
    if "kError" in str(st):
        raise RuntimeError(f"passModel failed: {st}")
    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    tmp = out.with_suffix(f".tmp{os.getpid()}.mps")
    h.writeModel(str(tmp))
    check(src, tmp, cp, rp)
    tmp.rename(out)
    return out


def check(src, permuted, cp, rp):
    """読み直して、逆の並べ替えで元と一致することを確かめる。"""
    _, a = _read(src)
    _, b = _read(permuted)
    n, m = a.num_col_, a.num_row_
    assert (b.num_col_, b.num_row_) == (n, m), "大きさが違う"
    assert str(a.sense_) == str(b.sense_), "向きが違う"
    assert a.offset_ == b.offset_, f"目的の定数が違う {a.offset_} {b.offset_}"
    for name, f in (("cost", "col_cost_"), ("lower", "col_lower_"), ("upper", "col_upper_")):
        x, y = list(getattr(a, f)), list(getattr(b, f))
        assert all(x[cp[j]] == y[j] for j in range(n)), f"列の {name} が違う"
    ia, ib = list(a.integrality_), list(b.integrality_)
    ia = ia or [highspy.HighsVarType.kContinuous] * n
    ib = ib or [highspy.HighsVarType.kContinuous] * n
    assert all(ia[cp[j]] == ib[j] for j in range(n)), "整数性が違う"
    for name, f in (("lower", "row_lower_"), ("upper", "row_upper_")):
        x, y = list(getattr(a, f)), list(getattr(b, f))
        assert all(x[rp[i]] == y[i] for i in range(m)), f"行の {name} が違う"
    ca, cb = _cols(a), _cols(b)
    for j in range(n):
        assert {rp[i]: v for i, v in cb[j].items()} == ca[cp[j]], f"列 {j} の非零が違う"


if __name__ == "__main__":
    make(sys.argv[1], int(sys.argv[2]), sys.argv[3])
    print("ok", sys.argv[3])
