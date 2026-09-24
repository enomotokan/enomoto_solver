//! Stuffing Singleton Columns (Gamrath, Koch, Martin, Miltenberger,
//! Weninger, "Progress in Presolving for Mixed Integer Programming", Math.
//! Prog. Comp. 7 (2015) 367-398, §3 — preprint ZIB-Report 13-48): fixes a
//! continuous singleton column (a variable whose column has exactly one
//! nonzero entry across every real row — the same structural condition
//! `colsingleton` and `dualfix`'s own lock-counting already key off) whose
//! objective sign is the *unfavorable* one for its row, by reasoning about
//! how much room the row itself has left rather than that one column in
//! isolation.
//!
//! ## Relationship to `dualfix`
//!
//! A singleton column's one nonzero entry gives it either `up_lock=0` or
//! `down_lock=0` (never both locked, since a lone entry can only resist
//! movement in *one* direction — see `dualfix`'s own docs on lock
//! counting). `dualfix` already fixes the case where the objective sign
//! *agrees* with the unlocked direction (`down_lock==0 && c>=0`, or
//! symmetrically `up_lock==0 && c<=0`) — no row reasoning needed there,
//! moving that far is free for every row simultaneously. This module picks
//! up exactly dualfix's residual: `a_rj>0 && c_j<0` (wants to move *up*,
//! the *locked* direction for a `<=` row with a positive entry) and its
//! mirror `a_rj<0 && c_j>0` (wants to move *down*, likewise locked). Moving
//! that way is no longer unconditionally safe — it uses up row `r`'s own
//! slack — but with only *one* row to reason about (the column's single
//! appearance), and every other flexible column in that row treated as a
//! knapsack of competing claims on the same slack, it can still often be
//! decided outright which of these columns end up at a bound in some
//! optimum, without solving the LP.
//!
//! `colsingleton`'s own docs note that a column singleton's *inequality*
//! case "needs a sign-based case analysis of whether the row is guaranteed
//! to bind" and is deferred; `dualpropagate` resolved the case where the
//! row provably always binds (a global, propagated argument). This module
//! resolves the opposite residual — the row does *not* provably bind, so
//! instead of asking "is there room," it asks "how much room, shared
//! between how many competing columns."
//!
//! ## The `a_rj>0, c_j<0` case (Algorithm 1 in the paper)
//!
//! For a `<=` row `r` (`sum_k a_rk x_k <= b_r` — this crate's `G` rows are
//! already in that sense, see `build_a_g`), let `J(r)` be every continuous
//! singleton column in `r` with `a_rj>0` and `c_j<0` — pushing any of them
//! up improves the objective but tightens the row. Define two activity
//! bounds that treat every `j in J(r)` as if still sitting at its lower
//! bound (the "hasn't been granted the push yet" baseline — moving one to
//! its upper bound later only ever *adds* to both, see the loop below) and
//! every other column at its ordinary best/worst case:
//!
//! ```text
//! Ũr = sum_{j in J(r)} a_rj*lb_j + sum_{k not in J(r), a_rk>0} a_rk*ub_k + sum_{k not in J(r), a_rk<0} a_rk*lb_k
//! L̃r = sum_{j in J(r)} a_rj*lb_j + sum_{k not in J(r), a_rk<0} a_rk*ub_k + sum_{k not in J(r), a_rk>0} a_rk*lb_k
//! ```
//!
//! Then process `J(r)` in ascending order of `c_j/a_rj` (most-negative
//! ratio — best objective gain per unit of row slack spent — first): for
//! `alpha = a_rj*ub_j`, `beta = a_rj*lb_j`,
//!
//!   - `alpha <= b_r - Ũr + beta` — even in the worst case for every column
//!     not yet decided, pushing this one to its upper bound still leaves
//!     `r` satisfiable, and doing so can only help the objective — fix
//!     `x_j = ub_j`.
//!   - otherwise, `b_r <= L̃r` — the row's minimum possible activity
//!     (everything not yet decided at its *most* row-tightening extreme)
//!     already saturates `b_r`, so this column (and, once triggered, every
//!     column processed after it — `L̃r` only ever grows from here) cannot
//!     move up at all — fix `x_j = lb_j`.
//!   - otherwise: genuinely undetermined by this row alone (the true LP
//!     optimum may sit at a fractional point, the classic continuous-
//!     knapsack shape the paper motivates this with) — left alone.
//!
//! `Ũr`/`L̃r` are then advanced by `alpha - beta` unconditionally (matching
//! the paper's own Algorithm 1 line-for-line, not just on the branch that
//! fires) before moving to the next column: once a column's slot has been
//! *considered* in this ratio order, every column considered after it must
//! reason about the worst case *including* the possibility that this one
//! ends up granted its push, whether or not it *was* — a column left
//! undetermined might still take any value up to `ub_j` in the eventual
//! LP solution, so later, less-attractive columns cannot assume otherwise
//! and stay sound.
//!
//! ## The mirror case, `a_rj<0, c_j>0`
//!
//! Here pushing `x_j` down (its objective-favorable direction) is what
//! tightens the row instead, so the roles of "granted" and "forced" swap:
//! the default is the *low* bound (cost-favorable), and a column only
//! moves to its *high* bound when the row's slack genuinely requires it.
//! Rather than re-deriving a second, easily-miscrossed set of `Ũ`/`L̃`
//! formulas and branch conditions from scratch, this module reduces the
//! mirror case to the one above by the substitution `y_j = ub_j - x_j`
//! (`y_j in [0, ub_j-lb_j]`, `y_j=0 <=> x_j=ub_j`, `y_j=ub_j-lb_j <=>
//! x_j=lb_j`) applied to every such column in the row at once: its new
//! coefficient is `-a_rj>0` and its new cost is `-c_j<0`, exactly the case
//! above, with `b_r` shifted by `-sum_j a_rj*ub_j` to absorb the constant
//! `a_rj*ub_j` terms the substitution introduces. Running the identical
//! [`stuffing_core`] on this transformed row and translating its `(j,
//! at "upper" in y)` results back (`y` at its upper bound means `x_j` at
//! its *lower* bound, and vice versa) is a pure change of variables — no
//! separate derivation to get subtly wrong, and no second implementation
//! to keep in sync with the first if either is ever revisited.
//!
//! ## What this does *not* attempt
//!
//! Both cases require `lb_j`/`ub_j` finite for every candidate column
//! (`lb`/`ub` for every *other* column in the row may still be infinite —
//! only ever contributing a consistent `+inf` to `Ũr` or `-inf` to `L̃r`,
//! never both in the same running sum, so no `inf - inf` ever arises
//! there): a candidate with an infinite bound has no finite `alpha`/`beta`
//! baseline to reason about, and — since it is a genuinely unbounded
//! column, dualfix's own unconditional fix would already have caught the
//! favorable-sign case — the unfavorable-sign case with an infinite bound
//! is simply left to whatever bound-tightening or the LP solve itself
//! resolves it into. This module also only ever fixes a column to one of
//! its *own* two bounds, never tightens a bound partway (unlike
//! `propagate`'s activity-bound tightening) — the paper's own algorithm
//! is a fixing procedure, not a bound-strengthening one.
//!
//! Both cases run independently per row using each column's real, current
//! `lb`/`ub` for every column *not* in the case currently being decided —
//! including a same-row column that belongs to the *other* case, which is
//! therefore treated as ordinary "else" uncertainty rather than folding in
//! whatever this call might otherwise have decided for it. This costs a
//! little potential extra reduction on the rare row with columns of both
//! signs, in exchange for each case's own result never depending on which
//! order the two are evaluated in.

use crate::sparse::{Csr, csr_row_iter};
use crate::params::presolve::TOL;

/// One continuous singleton column being decided for a single row, already
/// in the `a>0, c<0` orientation `stuffing_core` expects — `l`/`u` are
/// whichever bounds correspond to that orientation (a caller in the mirror
/// case passes the *transformed* `y`-space bounds, not `lb[j]`/`ub[j]`
/// directly; see the module docs).
struct Candidate {
    j: usize,
    a: f64,
    l: f64,
    u: f64,
    c: f64,
}

/// Algorithm 1 of the module docs, exactly: decides, for the `a>0, c<0`
/// orientation only, which of `candidates` can be fixed to their `u` or
/// `l` and which are left undetermined. `row_other` is every *other*
/// nonzero entry of this row (not a member of `candidates`) together with
/// its real, global `lb`/`ub` (via `k`) — used only for `Ũr`/`L̃r`'s
/// "else" sums, never reassigned. Returns `(j, true)` for "fix to `u`" and
/// `(j, false)` for "fix to `l`".
fn stuffing_core(row_other: &[(usize, f64)], b: f64, mut candidates: Vec<Candidate>, lb: &[f64], ub: &[f64]) -> Vec<(usize, bool)> {
    let mut tilde_u = 0.0f64;
    let mut tilde_l = 0.0f64;
    for cand in &candidates {
        tilde_u += cand.a * cand.l;
        tilde_l += cand.a * cand.l;
    }
    for &(k, ak) in row_other {
        if ak > 0.0 {
            tilde_u += ak * ub[k];
            tilde_l += ak * lb[k];
        } else if ak < 0.0 {
            tilde_u += ak * lb[k];
            tilde_l += ak * ub[k];
        }
    }

    // Ascending c/a: `a>0` always here, so this is just ascending `c` order
    // among equal `a`, but written as the general ratio the paper uses
    // since candidates in the same row can have any positive `a`.
    candidates.sort_by(|x, y| {
        let rx = x.c / x.a;
        let ry = y.c / y.a;
        rx.partial_cmp(&ry).unwrap_or(std::cmp::Ordering::Equal).then(x.j.cmp(&y.j))
    });

    let mut fixings = Vec::new();
    for cand in &candidates {
        let alpha = cand.a * cand.u;
        let beta = cand.a * cand.l;
        if alpha <= b - tilde_u + beta - TOL {
            fixings.push((cand.j, true));
        } else if b <= tilde_l - TOL {
            fixings.push((cand.j, false));
        }
        // Unconditional, per the module docs: whether or not this column
        // was just decided, every column considered after it must assume
        // it could still end up granted its push.
        tilde_l += alpha - beta;
        tilde_u += alpha - beta;
    }
    fixings
}

/// Returns `(j, value)` for every continuous singleton column this module
/// can fix outright. `real_g_rows`/`real_g_rhs` are `G`'s multi-variable
/// rows and right-hand sides only (the same "real rows" `dualfix` and
/// `propagate::extract_bounds` already separate out from `G`'s own
/// single-variable bound rows) — a column's box-bound rows never count as
/// its "one appearance" any more than they do for `dualfix`/`colsingleton`.
pub fn fix_singleton_columns(n: usize, a: &Csr, real_g_rows: &[Vec<(usize, f64)>], real_g_rhs: &[f64], c: &[f64], lb: &[f64], ub: &[f64]) -> Vec<(usize, f64)> {
    let mut in_equality = vec![false; n];
    let ar = a.as_ref();
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }

    // A column's single appearance across every real inequality row, once
    // it has one; seeing a second appearance anywhere disqualifies it
    // (`seen_twice`), matching `colsingleton`'s own non-cascading, "counts
    // computed once from the input" appearance tally.
    let mut occ: Vec<Option<(usize, f64)>> = vec![None; n];
    let mut seen_twice = vec![false; n];
    for (i, row) in real_g_rows.iter().enumerate() {
        for &(j, v) in row {
            if v == 0.0 || seen_twice[j] {
                continue;
            }
            if occ[j].is_some() {
                occ[j] = None;
                seen_twice[j] = true;
            } else {
                occ[j] = Some((i, v));
            }
        }
    }

    // Group each row's `a_rj>0, c_j<0` candidates (case A, decided
    // directly) and `a_rj<0, c_j>0` candidates (case B, decided via the
    // `y = ub-x` reduction to case A — see the module docs) separately;
    // a column can only ever land in one of the two, never both (its one
    // coefficient has a single sign).
    let mut case_a: Vec<Vec<(usize, f64)>> = vec![Vec::new(); real_g_rows.len()];
    let mut case_b: Vec<Vec<(usize, f64)>> = vec![Vec::new(); real_g_rows.len()];
    for j in 0..n {
        let Some((i, coeff)) = occ[j] else { continue };
        if in_equality[j] || !lb[j].is_finite() || !ub[j].is_finite() {
            continue;
        }
        if coeff > 0.0 && c[j] < -TOL {
            case_a[i].push((j, coeff));
        } else if coeff < 0.0 && c[j] > TOL {
            case_b[i].push((j, coeff));
        }
    }

    let mut fixed = Vec::new();
    for (i, row) in real_g_rows.iter().enumerate() {
        if !case_a[i].is_empty() {
            let members: std::collections::HashSet<usize> = case_a[i].iter().map(|&(j, _)| j).collect();
            let row_other: Vec<(usize, f64)> = row.iter().filter(|&&(k, _)| !members.contains(&k)).cloned().collect();
            let candidates: Vec<Candidate> = case_a[i].iter().map(|&(j, coeff)| Candidate { j, a: coeff, l: lb[j], u: ub[j], c: c[j] }).collect();
            for (j, at_upper) in stuffing_core(&row_other, real_g_rhs[i], candidates, lb, ub) {
                fixed.push((j, if at_upper { ub[j] } else { lb[j] }));
            }
        }
        if !case_b[i].is_empty() {
            let members: std::collections::HashSet<usize> = case_b[i].iter().map(|&(j, _)| j).collect();
            let row_other: Vec<(usize, f64)> = row.iter().filter(|&&(k, _)| !members.contains(&k)).cloned().collect();
            let mut b_shifted = real_g_rhs[i];
            let candidates: Vec<Candidate> = case_b[i]
                .iter()
                .map(|&(j, coeff)| {
                    b_shifted -= coeff * ub[j];
                    Candidate { j, a: -coeff, l: 0.0, u: ub[j] - lb[j], c: -c[j] }
                })
                .collect();
            for (j, at_upper_y) in stuffing_core(&row_other, b_shifted, candidates, lb, ub) {
                // `y` at its upper bound (`ub_j - lb_j`) means `x_j =
                // ub_j - (ub_j-lb_j) = lb_j`; `y` at its lower bound (`0`)
                // means `x_j = ub_j - 0 = ub_j` — the reverse of case A's
                // own `(j, at_upper) -> ub_j`/`lb_j` mapping above.
                fixed.push((j, if at_upper_y { lb[j] } else { ub[j] }));
            }
        }
    }
    fixed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    /// The paper's own motivating example: a fractional (continuous)
    /// knapsack, `3x0+2x1+x2<=4`, `x in [0,1]^3`, maximizing profits
    /// `9,4,1` (stored as costs `-9,-4,-1` to minimize). Ratios
    /// `c_j/a_j = -3,-2,-1`. Greedily filling by best ratio first: item 0
    /// fits entirely (uses 3 of 4), item 1 only half-fits (fractional in
    /// the true LP optimum — must stay undetermined), item 2 gets none of
    /// the remaining budget. Hand-verified against the classic fractional-
    /// knapsack optimum (`x0=1, x1=0.5, x2=0`), not just against this
    /// module's own arithmetic.
    #[test]
    fn fractional_knapsack_best_and_worst_ratio_items_get_fixed() {
        let n = 3;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, 3.0), (1, 2.0), (2, 1.0)];
        let rhs = vec![4.0];
        let c = vec![-9.0, -4.0, -1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![1.0, 1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        let mut by_j: std::collections::HashMap<usize, f64> = fixed.into_iter().collect();
        assert_eq!(by_j.remove(&0), Some(1.0), "best ratio: fully packed to its upper bound");
        assert_eq!(by_j.remove(&2), Some(0.0), "worst ratio: no budget left, fixed to lower bound");
        assert!(!by_j.contains_key(&1), "middle item is genuinely fractional (0.5) in the true optimum, must stay undetermined");
    }

    /// The exact mirror of the knapsack test above under `y_j = ub_j-x_j`
    /// (`a' = -a`, `c' = -c`, `b' = b - sum(a*ub)` — see the module docs'
    /// derivation): independently hand-verified (not just algebraically)
    /// against the same fractional-knapsack optimum, reframed as "which
    /// items must be forced up, at least cost, to make an already-violated
    /// row feasible again" — cheapest-per-unit-of-slack-gained (item 2,
    /// ratio 1) goes first, most expensive (item 0, ratio 3) last.
    #[test]
    fn mirror_case_reduces_correctly_via_the_y_substitution() {
        let n = 3;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, -3.0), (1, -2.0), (2, -1.0)];
        let rhs = vec![-2.0];
        let c = vec![9.0, 4.0, 1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![1.0, 1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        let mut by_j: std::collections::HashMap<usize, f64> = fixed.into_iter().collect();
        assert_eq!(by_j.remove(&0), Some(0.0), "most expensive to force up: stays at its cost-favorable lower bound");
        assert_eq!(by_j.remove(&2), Some(1.0), "cheapest to force up: fully forced to its upper bound");
        assert!(!by_j.contains_key(&1), "genuinely fractional (0.5) in the true optimum, must stay undetermined");
    }

    /// A column appearing in *two* real rows is not a singleton at all —
    /// moving it can affect a row this module never looks at, so it must
    /// never be fixed here regardless of how favorable its sign looks in
    /// either row alone.
    #[test]
    fn column_in_two_rows_is_not_a_singleton() {
        let n = 2;
        let a = csr_from_rows(&[], n);
        let rows = vec![vec![(0, 3.0), (1, 1.0)], vec![(0, 2.0)]];
        let rhs = vec![4.0, 10.0];
        let c = vec![-9.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &rows, &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// A column appearing in an equality row (even with a favorable-
    /// looking sign in some unrelated inequality row) is excluded outright
    /// — same exclusion `dualfix` applies, since an equality row locks
    /// both directions at once.
    #[test]
    fn column_in_equality_row_is_excluded() {
        let n = 2;
        let a = csr_from_rows(&[vec![(0, 1.0)]], n);
        let rows = vec![vec![(0, 3.0), (1, 1.0)]];
        let rhs = vec![4.0];
        let c = vec![-9.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &rows, &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// The objective-*favorable* sign combination (`a_rj>0, c_j>=0`) is
    /// `dualfix`'s own residual, not this module's — left untouched here
    /// regardless of the row's slack.
    #[test]
    fn favorable_sign_is_left_to_dualfix() {
        let n = 1;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, 3.0)];
        let rhs = vec![4.0];
        let c = vec![1.0];
        let lb = vec![0.0];
        let ub = vec![1.0];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// An unbounded "else" column sharing the row makes `Ũr` genuinely
    /// `+inf` (it could always claim however much slack is left), so the
    /// only sound conclusion is "never provably safe to push the candidate
    /// up" — no `alpha <= b - inf + beta` ever fires, and no `inf - inf`
    /// arithmetic ever happens either (see the module docs on why not).
    #[test]
    fn unbounded_other_column_blocks_the_upper_fixing() {
        let n = 2;
        let a = csr_from_rows(&[], n);
        // x1 unbounded above with a positive coefficient: the row's true
        // maximum activity is +inf, so nothing about x0's own slack can
        // ever be guaranteed.
        let row = vec![(0, 3.0), (1, 1.0)];
        let rhs = vec![100.0];
        let c = vec![-9.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![1.0, f64::INFINITY];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// A candidate with an infinite bound of its own has no finite
    /// baseline to reason about and must be skipped outright, not treated
    /// as some sentinel-large finite value.
    #[test]
    fn candidate_with_infinite_own_bound_is_skipped() {
        let n = 1;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, 3.0)];
        let rhs = vec![4.0];
        let c = vec![-9.0];
        let lb = vec![0.0];
        let ub = vec![f64::INFINITY];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn empty_input_fixes_nothing() {
        let n = 0;
        let a = csr_from_rows(&[], n);
        let fixed = fix_singleton_columns(n, &a, &[], &[], &[], &[], &[]);
        assert!(fixed.is_empty());
    }
}
