"""行・列を乱数で並べ替えた同値な MPS を作る: python perm.py 元.mps 種 出力.mps
(`import perm; perm.make(元, 種, 出力)` でも使う)。

HiGHS (highspy) で読み、行と列の順 (と各列の中の非零の順) を種 k の乱数で入れ替えた LP を `passModel` → `writeModel` で書く。
名前は一緒に並べ替える (名前で列を見分ける処理があっても同じ列を指す)。書いたあと読み直し、逆の並べ替えで元と
目的・境界・整数性・行の範囲・非零 (値は完全一致か、書き出しの丸めの相対 1e-14 以内)・目的の定数・向きが同じことを確かめる (違えば例外)。
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
    cn, rn = list(lp.col_names_), list(lp.row_names_)  # highspy は属性を読むたびに全体を複製するので 1 回だけ読む
    if len(cn) == n:
        nl.col_names_ = [cn[c] for c in cp]
    if len(rn) == m:
        nl.row_names_ = [rn[r] for r in rp]
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


def _eq(x, y):
    """同じか、MPS の書き出しの丸め (最後の桁、supportcase26 の行の境界など) の範囲 (相対 1e-14)。"""
    return x == y or abs(x - y) <= 1e-14 * max(1.0, abs(x))


def check(src, permuted, cp, rp):
    """読み直して、逆の並べ替えで元と一致することを確かめる。"""
    _, a = _read(src)
    _, b = _read(permuted)
    n, m = a.num_col_, a.num_row_
    assert (b.num_col_, b.num_row_) == (n, m), "大きさが違う"
    assert str(a.sense_) == str(b.sense_), "向きが違う"
    assert _eq(a.offset_, b.offset_), f"目的の定数が違う {a.offset_} {b.offset_}"
    for name, f in (("cost", "col_cost_"), ("lower", "col_lower_"), ("upper", "col_upper_")):
        x, y = list(getattr(a, f)), list(getattr(b, f))
        assert all(_eq(x[cp[j]], y[j]) for j in range(n)), f"列の {name} が違う"
    ia, ib = list(a.integrality_), list(b.integrality_)
    ia = ia or [highspy.HighsVarType.kContinuous] * n
    ib = ib or [highspy.HighsVarType.kContinuous] * n
    # HiGHS の MPS の書き出しは、非零も費用もない列の整数性を保たないことがある (physiciansched6-2 の 1 に固定の連続列 316 本が
    # 整数で書かれる)。境界が同じ整数に固定されていれば同値なので許す
    ca0 = _cols(a)
    cl0, cu0, co0 = list(a.col_lower_), list(a.col_upper_), list(a.col_cost_)
    def same_int(j):
        c = cp[j]
        if ia[c] == ib[j]: return True
        lo, up = cl0[c], cu0[c]
        return not ca0[c] and co0[c] == 0 and lo == up and float(lo).is_integer()
    assert all(same_int(j) for j in range(n)), "整数性が違う"
    for name, f in (("lower", "row_lower_"), ("upper", "row_upper_")):
        x, y = list(getattr(a, f)), list(getattr(b, f))
        assert all(_eq(x[rp[i]], y[i]) for i in range(m)), f"行の {name} が違う"
    ca, cb = _cols(a), _cols(b)
    for j in range(n):
        cj = {rp[i]: v for i, v in cb[j].items()}
        assert cj.keys() == ca[cp[j]].keys() and all(_eq(ca[cp[j]][r], v) for r, v in cj.items()), f"列 {j} の非零が違う"


if __name__ == "__main__":
    make(sys.argv[1], int(sys.argv[2]), sys.argv[3])
    print("ok", sys.argv[3])
