//! 番号 0..n の部分集合を、並び `set` と番号ごとの並びの中の位置 `pos` (なければ `usize::MAX`) で持つ形の、入れ・除きの共通部分
//! (analysis/unify_20261126 の 1 節の 8)。局所探索 (旧 FJ・`local_search`・Local-MIP・`shift_and_propagate`) の違反行の集合が、
//! 同じ 15 行を 4 か所に写して持っていた。並びの順は写しと同じ (除くときは末尾と入れ替える) なので、乱択で選ぶ行も変わらない。

/// `i` を集合に入れる (`on`) か除く。もう入っている・入っていないなら何もしない。除くときは並びの末尾の番号を `i` の位置に移す。
#[inline]
pub(crate) fn set_member(set: &mut Vec<usize>, pos: &mut [usize], i: usize, on: bool) {
    if on && pos[i] == usize::MAX {
        pos[i] = set.len();
        set.push(i);
    } else if !on && pos[i] != usize::MAX {
        let k = pos[i];
        let last = *set.last().unwrap();
        set.swap_remove(k);
        if last != i {
            pos[last] = k;
        }
        pos[i] = usize::MAX;
    }
}
