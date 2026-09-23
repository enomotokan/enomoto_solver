//! Column singletons in *inequality* (and ranged) rows — the case
//! [`colsingleton`](super::colsingleton) leaves alone (it only handles a
//! column whose one appearance is an equality row), resolved with the
//! sign-based argument Andersen & Andersen ("Presolving in Linear
//! Programming", 1995, §3.2) and HiGHS's own `colSingleton` use.
//!
//! ## Ranged rows are one row here
//!
//! `G` is all `<=` rows, so a ranged row `L <= r.x <= U` lives there as the
//! pair `r.x <= U`, `-r.x <= -L` — and every bound-preservation row pair
//! `colsingleton`/`doubleton` emit has exactly that shape. Counted naively a
//! column appearing only in such a row has *two* appearances and looks like
//! no singleton at all (Netlib `seba`: 86 of its 121 surviving columns were
//! exactly this). This pass first pairs every `G` row with its exact
//! negation and treats the pair as one logical row with both sides.
//!
//! ## The reduction
//!
//! Column `x_j` (cost `c_j != 0`, no equality-row appearance) appears in
//! exactly one logical row `L <= a x_j + r <= U`. The objective pushes `x_j`
//! in direction `d` (`+1` when `c_j < 0`) toward its box bound `t` on that
//! side, and moving that way pushes the row's activity toward exactly one
//! of its sides, `S` (`U` when `a*d > 0`, else `L`). With `[r_lo, r_hi]` the
//! range of `r` over the other columns' boxes:
//!
//! - **Fix** `x_j = t` when `t` is finite and `x_j = t` can never violate
//!   `S` whatever the others do: replacing `x_j` by `t` in any feasible point
//!   keeps it feasible (the opposite side only gets looser) and does not
//!   worsen the objective.
//! - **Tight row** when side `S` alone already implies `x_j`'s bound `t`
//!   (finite implied value, `t` on the far side of it): then every optimum
//!   has `S` holding with equality — if `S` were slack, `x_j` could move
//!   toward `t` (it is strictly short of `t`, or at `t` which forces `S`
//!   tight by the implication) and strictly improve the objective. The row
//!   becomes the equality `a x_j + r = S`, dropping its other side
//!   (implied by the equality), and `colsingleton` then substitutes `x_j`
//!   out, skipping the now-redundant bound-preservation row for `t`.
//!
//! One decision per logical row per call (a row turned into an equality is
//! no longer a candidate for its other columns this call); fixing several
//! columns of one row is fine since each test uses the other columns' full
//! boxes, a superset of whatever an earlier fix left them.

use std::collections::HashMap;

use crate::sparse::Csr;

const TOL: f64 = 1e-9;

pub struct IneqSingletonResult {
    /// `(column, value)` to fix.
    pub fixes: Vec<(usize, f64)>,
    /// `(g_row, partner)`: `real_rows[g_row]` (as `<=` row with its own rhs)
    /// becomes an equality; it and its paired opposite side (if any) leave `G`.
    pub implied_equalities: Vec<(usize, Option<usize>)>,
}

fn row_key(row: &[(usize, f64)], negate: bool) -> Vec<(usize, u64)> {
    row.iter().map(|&(k, v)| (k, (if negate { -v } else { v }).to_bits())).collect()
}

/// Range of `sum(v * x_k)` over the terms' boxes, excluding column `skip`.
fn others_range(row: &[(usize, f64)], skip: usize, lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (mut lo, mut hi) = (0.0f64, 0.0f64);
    for &(k, v) in row {
        if k == skip {
            continue;
        }
        let (a, b) = if v > 0.0 { (v * lb[k], v * ub[k]) } else { (v * ub[k], v * lb[k]) };
        lo += a;
        hi += b;
    }
    (if lo.is_nan() { f64::NEG_INFINITY } else { lo }, if hi.is_nan() { f64::INFINITY } else { hi })
}

pub fn run(n: usize, a: &Csr, real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], c: &[f64], lb: &[f64], ub: &[f64]) -> IneqSingletonResult {
    let mut out = IneqSingletonResult { fixes: Vec::new(), implied_equalities: Vec::new() };

    // Columns with any equality-row appearance are colsingleton's/aggregator's.
    let mut in_a = vec![false; n];
    {
        let ar = a.as_ref();
        for i in 0..ar.nrows() {
            for &k in ar.col_indices_of_row_raw(i) {
                in_a[k] = true;
            }
        }
    }

    // Pair each row with its exact negation.
    let mut index: HashMap<Vec<(usize, u64)>, usize> = HashMap::with_capacity(real_rows.len());
    for (i, row) in real_rows.iter().enumerate() {
        index.entry(row_key(row, false)).or_insert(i);
    }
    let mut partner: Vec<Option<usize>> = vec![None; real_rows.len()];
    for (i, row) in real_rows.iter().enumerate() {
        if partner[i].is_some() {
            continue;
        }
        if let Some(&p) = index.get(&row_key(row, true)) {
            if p != i && partner[p].is_none() {
                partner[i] = Some(p);
                partner[p] = Some(i);
            }
        }
    }

    // Logical rows: the lower-indexed row of each pair stands for both.
    let mut logical_count = vec![0usize; n];
    let mut logical_row_of = vec![usize::MAX; n];
    for (i, row) in real_rows.iter().enumerate() {
        if matches!(partner[i], Some(p) if p < i) {
            continue;
        }
        for &(k, v) in row {
            if v != 0.0 {
                logical_count[k] += 1;
                logical_row_of[k] = i;
            }
        }
    }

    let mut row_done = vec![false; real_rows.len()];
    for j in 0..n {
        if in_a[j] || logical_count[j] != 1 || c[j] == 0.0 || lb[j] == ub[j] {
            continue;
        }
        let i = logical_row_of[j];
        if row_done[i] {
            continue;
        }
        let row = &real_rows[i];
        let Some(&(_, aj)) = row.iter().find(|&&(k, _)| k == j) else { continue };
        let upper = real_rhs[i];
        let lower = partner[i].map_or(f64::NEG_INFINITY, |p| -real_rhs[p]);
        let (r_lo, r_hi) = others_range(row, j, lb, ub);
        let d = if c[j] < 0.0 { 1.0 } else { -1.0 };
        let target = if d > 0.0 { ub[j] } else { lb[j] };
        let toward_upper = aj * d > 0.0;
        let tol = |x: f64| TOL * (1.0 + x.abs());

        // Fix: moving all the way to `target` never violates side S.
        if target.is_finite() {
            let never_violates = if toward_upper {
                aj * target + r_hi <= upper + tol(upper)
            } else {
                lower == f64::NEG_INFINITY || aj * target + r_lo >= lower - tol(lower)
            };
            if never_violates {
                out.fixes.push((j, target));
                continue;
            }
        }

        // Tight row: side S alone implies x_j's bound in direction d.
        let (side, side_rhs, residual, g_row) = if toward_upper {
            (upper, upper, r_lo, Some(i))
        } else {
            (lower, lower, r_hi, partner[i])
        };
        let Some(g_row) = g_row else { continue };
        if !side.is_finite() || !residual.is_finite() {
            continue;
        }
        // a x_j <= S - r_lo (upper side) or a x_j >= S - r_hi (lower side).
        let implied = (side_rhs - residual) / aj;
        let implies_target = if d > 0.0 { implied <= target + tol(target) } else { implied >= target - tol(target) };
        if implies_target {
            let other = if g_row == i { partner[i] } else { Some(i) };
            out.implied_equalities.push((g_row, other));
            row_done[i] = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    fn empty_a(n: usize) -> Csr {
        csr_from_rows(&[], n)
    }

    #[test]
    fn fixes_when_the_row_can_never_bind() {
        // min -x0, 0 <= x0 <= 1, x0 + x1 <= 5, x1 in [0,1]: x0 = 1 never violates.
        let r = run(2, &empty_a(2), &[vec![(0, 1.0), (1, 1.0)]], &[5.0], &[-1.0, 0.0], &[0.0, 0.0], &[1.0, 1.0]);
        assert_eq!(r.fixes, vec![(0, 1.0)]);
        assert!(r.implied_equalities.is_empty());
    }

    #[test]
    fn ranged_pair_is_one_row_and_its_upper_side_becomes_tight() {
        // min -x0, x0 in [0,10], 1 <= x0 + x1 <= 3 (as a pair), x1 in [0,1]:
        // the upper side implies x0 <= 3 <= 10, so it is tight at every optimum.
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]];
        let r = run(2, &empty_a(2), &rows, &[3.0, -1.0], &[-1.0, 0.0], &[0.0, 0.0], &[10.0, 1.0]);
        assert!(r.fixes.is_empty());
        assert_eq!(r.implied_equalities, vec![(0, Some(1))]);
    }

    #[test]
    fn lower_side_becomes_tight_when_cost_pushes_down() {
        // min x0 (pushes down), x0 in [0,10], 1 <= x0 + x1 <= 3, x1 in [0,1]:
        // the lower side implies x0 >= 0 = lb only when 1 - r_hi = 0 >= 0.
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]];
        let r = run(2, &empty_a(2), &rows, &[3.0, -1.0], &[1.0, 0.0], &[0.0, 0.0], &[10.0, 1.0]);
        assert!(r.fixes.is_empty());
        assert_eq!(r.implied_equalities, vec![(1, Some(0))]);
    }

    #[test]
    fn neither_when_the_box_and_the_row_both_can_bind() {
        // min -x0, x0 in [0,2], x0 + x1 <= 2.5, x1 in [0,1]: x0 = 2 violates when
        // x1 = 1, and the row only implies x0 <= 2.5 > 2 — no reduction.
        let r = run(2, &empty_a(2), &[vec![(0, 1.0), (1, 1.0)]], &[2.5], &[-1.0, 0.0], &[0.0, 0.0], &[2.0, 1.0]);
        assert!(r.fixes.is_empty() && r.implied_equalities.is_empty());
    }

    #[test]
    fn equality_row_columns_are_left_to_colsingleton() {
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let r = run(2, &a, &[vec![(0, 1.0), (1, 1.0)]], &[5.0], &[-1.0, 0.0], &[0.0, 0.0], &[1.0, 1.0]);
        assert!(r.fixes.is_empty() && r.implied_equalities.is_empty());
    }
}
