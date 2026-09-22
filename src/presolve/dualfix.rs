//! DualFix (Achterberg, Bixby, Gu, Rothberg, Weninger, "Presolve
//! Reductions in Mixed Integer Programming", §4.4): a variable whose
//! objective cost prefers one direction can be fixed to the corresponding
//! bound outright — no simplex iteration needed — provided *no* real
//! constraint would resist moving it that way. "Real" excludes the
//! variable's own box-bound rows (folded into `G` by `build_a_g`): those
//! aren't independent constraints, they're the bounds themselves, so
//! counting them would make every variable look locked in both
//! directions by its own bounds and this reduction would never fire.
//!
//! The decision is "up-lock"/"down-lock" counting: for each real row, an
//! entry locks the direction that would move that row's activity *closer*
//! to violating it. `G`'s rows are already normalized to `<=` sense (see
//! `build_a_g`), so within them a positive entry always locks "up" and a
//! negative entry always locks "down". An equality row locks *both*
//! directions at once (moving either way breaks it, absent compensation
//! from other variables) — a variable appearing in any equality row is
//! disqualified outright, not analyzed further.
//!
//! If a variable's total down-lock count is `0` and its cost is `>= 0`
//! (minimization wants it small, and nothing stops it from going all the
//! way down), it is fixed to its lower bound; symmetrically for an
//! up-lock count of `0` and cost `<= 0`.

use crate::sparse::{Csr, csr_row_iter};
const TOL: f64 = 1e-9;

/// Returns `(j, value)` for every variable that can be fixed outright.
/// `real_g_rows` is `G`'s multi-variable rows only — the same "real rows"
/// list `propagate::extract_bounds` already separates out from `G`'s
/// single-variable bound rows, reused here instead of re-deriving it.
pub fn fix_dominated_variables(
    n: usize,
    a: &Csr,
    real_g_rows: &[Vec<(usize, f64)>],
    c: &[f64],
    lb: &[f64],
    ub: &[f64],
) -> Vec<(usize, f64)> {
    let mut up_lock = vec![0usize; n];
    let mut down_lock = vec![0usize; n];
    let mut in_equality = vec![false; n];

    let ar = a.as_ref();
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }

    for row in real_g_rows {
        for &(j, v) in row {
            if v > 0.0 {
                up_lock[j] += 1;
            } else if v < 0.0 {
                down_lock[j] += 1;
            }
        }
    }

    let mut fixed = Vec::new();
    for j in 0..n {
        if in_equality[j] {
            continue;
        }
        if down_lock[j] == 0 && c[j] >= -TOL && lb[j].is_finite() {
            fixed.push((j, lb[j]));
        } else if up_lock[j] == 0 && c[j] <= TOL && ub[j].is_finite() {
            fixed.push((j, ub[j]));
        }
    }
    fixed
}
