//! FreeVar (general free-variable elimination via equality rows): a
//! variable with both bounds infinite (`l=-inf`, `u=+inf` — a genuine free
//! variable, not a finite sentinel later clamped to a large-but-finite
//! substitute) is eliminated from `A x = b` by plain Gaussian elimination
//! through any equality row it still appears in, generalizing
//! [`crate::presolve::colsingleton`]'s "appears in exactly one row"
//! restriction to "appears in any number of rows": this module's pivot
//! search picks, for each free variable in turn, whichever of its
//! remaining rows has the largest-magnitude coefficient (for numerical
//! stability, the same reason `colsingleton` guards its own single-row
//! pivot with `SUBSTITUTION_PIVOT_RATIO`), solves that row for the
//! variable, and folds it into every *other* row (and the objective) that
//! still references it — exactly the fold `colsingleton` already performs
//! against its own single owning row, just repeated across as many rows as
//! the variable actually appears in, and iterated (since folding one free
//! variable's pivot row into the rest can drop another free variable's
//! last remaining appearance, exposing it as its own eliminable case) until
//! no free variable has any remaining `A`-row appearance left.
//!
//! Unlike a finite-bound elimination ([`crate::presolve::colsingleton`],
//! [`crate::presolve::doubleton`]), a truly free variable never needs a
//! replacement box row on the remaining variables to preserve its own
//! bounds elsewhere — those bounds are already vacuous (`-inf <= r <=
//! +inf` constrains nothing) — so eliminating it is unconditionally free
//! of the "must preserve the eliminated variable's own bounds somewhere"
//! bookkeeping those modules carry (see `colsingleton`'s own docs for why
//! that bookkeeping exists at all, and why it is the reason this module
//! never has to emit anything analogous to its `extra_g_rows`/`extra_h`).
//!
//! Like `colsingleton`'s own elimination, no constant term folded out of
//! the objective (`c_j * rhs_i / coeff`, the part of `c_j * x_j` that does
//! *not* depend on any other variable) needs to be tracked here: nothing
//! downstream of presolve ever trusts a reduced sub-problem's own internal
//! objective value directly — the true objective is always recomputed by
//! evaluating the *original* cost vector against the fully reconstructed
//! `x` (every substitution's `value()` applied, in reverse discovery
//! order — see `simplex.rs::unscale_result`), so a constant shift that
//! would apply identically to every candidate solution can simply be
//! dropped without changing which `x` is optimal.
//!
//! ## Leftover free variables
//!
//! Once every free variable with a remaining `A`-row appearance has been
//! eliminated, two further cases are resolved directly against
//! `real_rows`/`real_rhs` (the real, multi-variable inequality rows —
//! `G`'s own single-variable box rows are excluded, same as everywhere
//! else in this crate; both start out read-only inputs but this module now
//! *does* remove a row from them when it eliminates the one free variable
//! that row exists to bound, so [`FreeVarResult`] hands back the
//! post-removal versions for the caller to use from here on):
//!
//! - **No remaining appearance anywhere** (never had one, or lost its last
//!   one to a fold above): resolved purely by objective coefficient,
//!   following the paper's own §4.1 case split — a nonzero one means
//!   moving it in the direction opposite that coefficient's sign strictly
//!   improves the objective without bound (the whole problem is unbounded
//!   — [`FreeVarResult::unbounded`]), a zero one means fixing it to any
//!   finite value costs nothing (`0`, arbitrarily).
//! - **Exactly one inequality-row appearance, no other row anywhere**: the
//!   generalization of the case above — that single row, say (after `G`'s
//!   own `<=` normalization) `a_j x_j + rest <= h`, is the *only* thing
//!   constraining `x_j`, so it can be isolated exactly the way an equality
//!   row already is (`x_j = (h - rest) / a_j`) *provided* moving `x_j`
//!   toward that boundary is what the objective wants — i.e. provided
//!   `a_j` and `c_j` don't share a sign (a shared sign means the row only
//!   bounds `x_j` on the side the objective has no interest in reaching,
//!   so it can be pushed the *other*, genuinely unbounded way instead,
//!   exactly the "no appearance at all" case above but discovered through
//!   one still-live row rather than zero). When eligible, the row is
//!   genuinely redundant once `x_j` is gone — nothing else references
//!   `x_j`, by this case's own "exactly one appearance" precondition, so
//!   unlike an `A`-row elimination there is nothing left to fold this row
//!   into — and is dropped from `real_rows`/`real_rhs` outright, cascading
//!   exactly like the `A`-row loop above (dropping one variable's own row
//!   can reduce a *different* free variable sharing that row down to zero
//!   or one appearance in turn).
//!
//! A free variable that still has two or more inequality-row appearances
//! (and none in `A`) is left exactly as-is (still free, not resolved) — a
//! documented residual case for [`crate::simplex`]'s `x_j = x_j^+ - x_j^-`
//! split to handle instead (see `simplex.rs::build_std_form_presolved`'s
//! own docs), rather than something this module can soundly eliminate: no
//! single one of several rows can be isolated for `x_j` the way exactly
//! one can, and folding would require picking one row to solve while
//! leaving `x_j` live in the rest, which is exactly the "must preserve
//! meaning in every other row" bookkeeping this module's own docs above
//! note a truly free variable is supposed to be exempt from.

use crate::presolve::colsingleton::Substitution;
use crate::sparse::{Csr, SparseAccum, axpy_row, csr_from_rows, csr_rows};
use crate::params::presolve::{SUBSTITUTION_PIVOT_RATIO, TOL};

pub struct FreeVarResult {
    pub a: Csr,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    pub substitutions: Vec<Substitution>,
    /// Leftover free variables resolved by objective coefficient alone —
    /// `(var, 0.0)` pairs, threaded through by the caller exactly like
    /// `dualfix`'s own `(usize, f64)` fixes (`lb[j] = ub[j] = value`).
    pub fixed: Vec<(usize, f64)>,
    /// `true` iff some leftover free variable (no remaining `A`-row
    /// appearance, and either no inequality-row appearance or exactly one
    /// whose sign combination doesn't let it be isolated — see the module
    /// docs' "Leftover free variables" section) has an objective
    /// coefficient that makes it genuinely unbounded — the problem is
    /// unbounded and the caller must stop immediately without trusting
    /// this result's other fields (`fixed`/`real_rows`/`real_rhs` are left
    /// at whatever partial state they reached and `a`/`b`/`c` reflect only
    /// the eliminations found before the unbounded variable was
    /// discovered, none of which the caller needs once it is reporting
    /// `Status::Unbounded`).
    pub unbounded: bool,
    /// `real_rows`/`real_rhs` with every row this module eliminated (the
    /// "exactly one inequality-row appearance" case above) removed — the
    /// caller must use these, not its own original copies, for anything
    /// downstream (`propagate::rebuild_g`, and the final
    /// `ExtendedPresolveResult` fields `simplex.rs`/`interior_point.rs`
    /// both read `g_rows`/`g_rhs` from).
    pub real_rows: Vec<Vec<(usize, f64)>>,
    pub real_rhs: Vec<f64>,
}

fn appears_in(real_rows: &[Vec<(usize, f64)>], j: usize) -> bool {
    real_rows.iter().any(|row| row.iter().any(|&(k, v)| k == j && v != 0.0))
}

/// Eliminates every free structural variable (`0..n`) reachable through
/// `A`'s own rows, folding each one's pivot row into every other row (and
/// the objective) that still references it; then eliminates every free
/// variable left with exactly one `real_rows` appearance and a favorable
/// sign combination, dropping that row outright (see the module docs'
/// "Leftover free variables" section for both).
pub fn eliminate_free_variables(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> FreeVarResult {
    let is_free: Vec<bool> = (0..n).map(|j| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY).collect();
    if !is_free.iter().any(|&f| f) {
        return FreeVarResult {
            a: a.clone(),
            b: b.to_vec(),
            c: c.to_vec(),
            substitutions: Vec::new(),
            fixed: Vec::new(),
            unbounded: false,
            real_rows: real_rows.to_vec(),
            real_rhs: real_rhs.to_vec(),
        };
    }

    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    // One sparse accumulator for every row fold this pass performs —
    // see `crate::sparse::SparseAccum`'s own docs for why the merge is
    // not a per-row `BTreeMap`.
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    let mut eliminated = vec![false; n];
    let mut substitutions = Vec::new();
    // Mutated by *both* passes below, not just the inequality one: a
    // variable the `A`-row loop eliminates can easily also appear in a
    // `real_rows` entry (nothing before this module ever removes a
    // structural variable from `G`'s own multi-variable rows just because
    // some *other* module eliminated it from `A`), and that appearance
    // must be folded away too — left alone, it would still reference a
    // column this function is about to report as eliminated, which every
    // downstream reader treats as *fixed to `0`* (the pinning convention
    // `presolve.rs` applies to every `Substitution::var`), silently
    // replacing whatever that row's true dependence on `x_j` was with
    // "`x_j` is exactly `0`" — confirmed to actually corrupt real Netlib
    // instances (`perold`, `pilot4`) into a false `Infeasible` (or, at a
    // looser `SUBSTITUTION_PIVOT_RATIO`, a wrong finite objective) before
    // this fold existed.
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();
    // Set for a free variable whose best available `A`-row pivot still
    // fails `SUBSTITUTION_PIVOT_RATIO` (see that constant's own docs) —
    // excluded from the rest of *this* pass (nothing changed about its own
    // candidate rows, so re-selecting it would just fail the same check
    // forever) and, since it still has a live, un-eliminated `A`-row
    // appearance by construction, from the inequality pass below too (that
    // pass's own "zero `A`-row appearance" precondition would otherwise be
    // silently violated for it) and from the final leftover resolution
    // (which must not treat a variable that still owes an equation as
    // "genuinely unconstrained").
    let mut skip_a = vec![false; n];

    loop {
        // Every remaining free variable's appearance rows, recomputed
        // fresh each pass since folding one free variable's pivot row into
        // the rest can drop a *different* free variable's last remaining
        // appearance (never add one — a fold only ever cancels the pivot
        // column, it cannot introduce a free variable where there was
        // none), exposing it as its own eliminable case next pass.
        let mut appearances: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, row) in a_rows.iter().enumerate() {
            for &(j, v) in row {
                if is_free[j] && !eliminated[j] && v != 0.0 {
                    appearances[j].push(i);
                }
            }
        }
        let Some(j) = (0..n).find(|&j| is_free[j] && !eliminated[j] && !skip_a[j] && !appearances[j].is_empty()) else {
            break;
        };

        let mut best_i = appearances[j][0];
        let mut best_coeff = 0.0f64;
        for &i in &appearances[j] {
            let coeff = a_rows[i].iter().find(|&&(k, _)| k == j).unwrap().1;
            if coeff.abs() > best_coeff.abs() {
                best_coeff = coeff;
                best_i = i;
            }
        }
        let coeff = best_coeff;
        let pivot_row = a_rows[best_i].clone();
        let row_max = pivot_row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < tunable!("ENOMOTO_T_FV_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
            skip_a[j] = true;
            continue;
        }
        let rhs_i = b[best_i];
        let terms: Vec<(usize, f64)> = pivot_row.iter().filter(|&&(k, _)| k != j).copied().collect();

        for &i2 in &appearances[j] {
            if i2 == best_i {
                continue;
            }
            let a_i2j = a_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            b[i2] -= factor * rhs_i;
        }

        // Same fold, into every `real_rows` entry `j` still appears in
        // (see this function's own `real_rows`/`real_rhs` docs above for
        // why this is required, not optional) — `real_rows` rows are never
        // removed here (only the inequality pass below ever drops a row
        // outright, when eliminating the one variable that row exists to
        // bound), just rewritten to no longer reference `j`.
        for i2 in 0..real_rows.len() {
            let Some(&(_, a_i2j)) = real_rows[i2].iter().find(|&&(k, _)| k == j) else {
                continue;
            };
            let factor = a_i2j / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            real_rhs[i2] -= factor * rhs_i;
        }

        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        eliminated[j] = true;
        a_rows.remove(best_i);
        b.remove(best_i);
    }

    // Every free variable reaching this point either has zero remaining
    // `A`-row appearances, or is `skip_a`-marked (a live `A`-row appearance
    // the loop above deliberately left untouched — excluded below too, see
    // `skip_a`'s own docs) — so, for everything actually eligible here,
    // only `real_rows` appearances matter. A variable with exactly one is
    // eliminable in place, exactly like a single-row `A` equality above,
    // *provided* isolating it there is what the objective actually wants
    // (see the module docs) *and* the pivot clears the same
    // `SUBSTITUTION_PIVOT_RATIO` guard the `A`-row loop above does,
    // recorded in `skip_g` for the same reason `skip_a` exists. Cascaded
    // the same way, since dropping one variable's own row can reduce a
    // *different* free variable sharing that row down to zero or one
    // appearance in turn.
    let mut skip_g = vec![false; n];
    loop {
        let mut appearances: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, row) in real_rows.iter().enumerate() {
            for &(j, v) in row {
                if is_free[j] && !eliminated[j] && !skip_a[j] && v != 0.0 {
                    appearances[j].push(i);
                }
            }
        }
        let Some(j) = (0..n).find(|&j| is_free[j] && !eliminated[j] && !skip_a[j] && !skip_g[j] && appearances[j].len() == 1) else {
            break;
        };
        let i = appearances[j][0];
        let row = real_rows[i].clone();
        let coeff = row.iter().find(|&&(k, _)| k == j).unwrap().1;
        let row_max = row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < tunable!("ENOMOTO_T_FV_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
            skip_g[j] = true;
            continue;
        }
        let cj = c[j];

        // `coeff` and `cj` sharing a (strict, non-`TOL`-noise) sign means
        // this row only bounds `x_j` on the side the objective has no
        // interest in reaching — pushed the other way, `x_j` is genuinely
        // unbounded (see the module docs' derivation). `cj` within `TOL`
        // of zero never triggers this: the objective doesn't care which
        // feasible value `x_j` takes, so isolating it at this row's own
        // boundary below is always valid then.
        if cj.abs() > TOL && (coeff > 0.0) == (cj > 0.0) {
            return FreeVarResult { a: csr_from_rows(&a_rows, n), b, c, substitutions, fixed: Vec::new(), unbounded: true, real_rows, real_rhs };
        }

        let rhs_i = real_rhs[i];
        let terms: Vec<(usize, f64)> = row.iter().filter(|&&(k, _)| k != j).copied().collect();

        // Same cost fold as the `A`-row loop above, and for the same
        // reason (see its own comment): `x_j`'s objective contribution
        // `cj * x_j` becomes, after substituting `x_j = (rhs_i -
        // sum(terms)) / coeff`, a constant (dropped, never tracked — see
        // the module docs) plus `-factor * a_ik` folded onto each `terms`
        // column's own cost. Skipped only when `cj == 0` exactly, where
        // there is nothing to fold (`terms` costs are already correct).
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        eliminated[j] = true;
        real_rows.remove(i);
        real_rhs.remove(i);
    }

    let mut fixed = Vec::new();
    for j in 0..n {
        if !is_free[j] || eliminated[j] || skip_a[j] || appears_in(&real_rows, j) {
            continue;
        }
        if c[j].abs() > TOL {
            return FreeVarResult { a: csr_from_rows(&a_rows, n), b, c, substitutions, fixed: Vec::new(), unbounded: true, real_rows, real_rhs };
        }
        fixed.push((j, 0.0));
    }

    FreeVarResult { a: csr_from_rows(&a_rows, n), b, c, substitutions, fixed, unbounded: false, real_rows, real_rhs }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csr(rows: &[Vec<(usize, f64)>], n: usize) -> Csr {
        csr_from_rows(rows, n)
    }

    #[test]
    fn no_free_variables_is_a_no_op() {
        let a = csr(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![1.0, 2.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        assert!(r.substitutions.is_empty());
        assert!(r.fixed.is_empty());
        assert_eq!(r.b, b);
        assert_eq!(r.c, c);
    }

    #[test]
    fn eliminates_free_variable_appearing_in_two_rows() {
        // x0 free, x1,x2 in [0,10]. Rows: x0 + x1 = 5, x0 - x2 = 1.
        // Eliminating x0 via row 0 (x0 = 5 - x1) folds into row 1:
        // (5 - x1) - x2 = 1  =>  -x1 - x2 = -4.
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0, 0.0];
        let ub = vec![f64::INFINITY, 10.0, 10.0];
        let r = eliminate_free_variables(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        // x0's cost (3.0) folds into x1's cost via -3*1/1 = -3.
        assert_eq!(r.c[1], 1.0 - 3.0);
        assert_eq!(r.c[2], 1.0);
        assert_eq!(r.a.nrows(), 1);
        // Recover x0 given x1=2, x2=1: x0 = 5 - 2 = 3.
        let x = vec![3.0, 2.0, 1.0];
        assert!((r.substitutions[0].value(&x) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn a_row_elimination_folds_into_a_shared_real_row_too() {
        // x0 free, x1,x2 in [0,10]. x0 == x1 (an `A` equality row -- x0's
        // *only* `A`-row appearance, so it's eliminated via it), and
        // separately x0 + x2 <= 5 (a `real_rows` inequality -- x0 also
        // appears here). Regression test for a real bug: the `A`-row loop
        // used to fold a variable's elimination into every *other* `A` row
        // it appeared in, but never into a `real_rows` entry it happened
        // to share -- silently leaving that row referencing a column this
        // function reports as eliminated, which `presolve.rs` then pins to
        // `lb[var]=ub[var]=0.0`, so the row got read downstream as if
        // `x0` were fixed to `0` instead of `x1`. Confirmed to actually
        // corrupt real Netlib instances (`perold`, `pilot4`) into a false
        // `Infeasible` before this fold was added.
        let a = csr(&[vec![(0, 1.0), (1, -1.0)]], 3);
        let b = vec![0.0];
        let c = vec![0.0, -2.0, -1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0, 0.0];
        let ub = vec![f64::INFINITY, 10.0, 10.0];
        let real_rows = vec![vec![(0, 1.0), (2, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(3, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        // The shared row must no longer mention x0 (column 0) at all, and
        // must instead read exactly x1 + x2 <= 5 (x0's coefficient (1.0)
        // folded onto x1 via the substitution x0 = 0 + 1*x1).
        assert_eq!(r.real_rows.len(), 1);
        assert_eq!(r.real_rows[0], vec![(1, 1.0), (2, 1.0)]);
        assert_eq!(r.real_rhs, vec![5.0]);
    }

    #[test]
    fn leftover_free_variable_with_nonzero_cost_is_unbounded() {
        // x0 free, appears nowhere.
        let a = csr(&[vec![(1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.unbounded);
    }

    #[test]
    fn leftover_free_variable_with_zero_cost_is_fixed() {
        let a = csr(&[vec![(1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![0.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        assert_eq!(r.fixed, vec![(0, 0.0)]);
    }

    #[test]
    fn free_variable_in_two_inequality_rows_is_left_alone() {
        // x0 free in two separate inequality rows -- neither alone
        // determines it, and no single one can be isolated the way
        // exactly one can, so it stays exactly as-is (the residual case
        // `simplex.rs`'s x_j = x_j^+ - x_j^- split now handles).
        let a = csr(&[vec![(1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (1, -1.0)]];
        let real_rhs = vec![5.0, 5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert!(r.fixed.is_empty());
        assert!(r.substitutions.is_empty());
        assert_eq!(r.real_rows, real_rows);
        assert_eq!(r.real_rhs, real_rhs);
    }

    #[test]
    fn free_variable_in_one_inequality_row_with_favorable_sign_is_substituted() {
        // x0 free, x1 in [0,10]; single inequality row x0 + x1 <= 5, cost
        // -x0 + x1 (minimizing wants x0 *large*, i.e. maximized -- the row
        // caps x0 from above, exactly the direction the objective wants
        // capped, so it's eligible: coeff(+1) and c[0](-1) have opposite
        // signs). Substituted at the row's own boundary: x0 = 5 - x1, and
        // the row itself is dropped as redundant (nothing else references
        // x0, by the "exactly one appearance" precondition).
        let a = csr(&[], 2);
        let b = vec![];
        let c = vec![-1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert!(r.fixed.is_empty());
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        assert!(r.real_rows.is_empty(), "the now-redundant row must be dropped, real_rows={:?}", r.real_rows);
        assert!(r.real_rhs.is_empty());
        // x0's cost (-1) folds into x1's via -(-1)*1/1 = +1: c[1] = 1+1 = 2.
        assert_eq!(r.c[0], 0.0);
        assert_eq!(r.c[1], 2.0);
        // Recover x0 given x1=2: x0 = 5 - 2 = 3.
        let x = vec![3.0, 2.0];
        assert!((r.substitutions[0].value(&x) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn free_variable_in_one_inequality_row_with_unfavorable_sign_is_unbounded() {
        // Same row as above, but cost x0 + x1 (minimizing wants x0
        // *small*) -- the row's own coeff(+1) and c[0](+1) share a sign,
        // so the row only caps x0 from *above*, exactly the direction the
        // objective has no interest in; pushed the other way (toward
        // -infinity) nothing stops it.
        let a = csr(&[], 2);
        let b = vec![];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(r.unbounded);
    }

    #[test]
    fn free_variable_in_one_inequality_row_with_zero_cost_is_substituted() {
        // Same row again, but c[0] == 0: the objective doesn't care which
        // feasible value x0 takes, so isolating it at the row's own
        // boundary is always valid regardless of `coeff`'s sign.
        let a = csr(&[], 2);
        let b = vec![];
        let c = vec![0.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert_eq!(r.substitutions.len(), 1);
        assert!(r.real_rows.is_empty());
    }

    #[test]
    fn cascading_elimination_through_shared_inequality_row() {
        // x0, x1 free; x2 in [0,10]. Row A: x0 + x1 <= 5 (x0's *only*
        // appearance; x1's first). Row B: x1 + x2 <= 3 (x1's second
        // appearance; x2's only, but x2 isn't free so it never drives
        // elimination itself). x1 starts at appearance count 2, so pass 1
        // can only reach x0 (count 1 already): substituted (coeff(+1) vs
        // c[0]=-1 opposite signs), folding c[0]'s cost onto x1's own
        // (factor -1/1=-1: c[1] -= -1*1 => -2.0+1.0 = -1.0) and dropping
        // Row A. That drop is what cascades: x1's appearance count falls
        // to 1 (Row B only), so pass 2 reaches it in turn, using its own
        // *post-fold* cost (-1.0, not the original -2.0 -- exercising that
        // the fold from pass 1 is what pass 2's own sign check must see)
        // against Row B's coeff(+1): opposite signs again, substitute,
        // folding onto x2's cost (factor -1/1=-1: c[2] -= -1*1 =>
        // 0.5+1.0 = 1.5) and dropping Row B too.
        let a = csr(&[], 3);
        let b = vec![];
        let c = vec![-1.0, -2.0, 0.5];
        let lb = vec![f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)]];
        let real_rhs = vec![5.0, 3.0];
        let r = eliminate_free_variables(3, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert!(r.fixed.is_empty());
        assert_eq!(r.substitutions.len(), 2);
        assert_eq!(r.substitutions[0].var, 0);
        assert_eq!(r.substitutions[1].var, 1);
        assert!(r.real_rows.is_empty());
        assert!(r.real_rhs.is_empty());
        assert_eq!(r.c, vec![0.0, 0.0, 1.5]);
        // Recover given x2=2: x1 = 3-2=1, x0 = 5-1=4. Both rows exactly
        // tight, as the boundary substitution intends.
        let x = vec![4.0, 1.0, 2.0];
        assert!((r.substitutions[1].value(&x) - 1.0).abs() < 1e-9, "x1 sub: {}", r.substitutions[1].value(&x));
        assert!((r.substitutions[0].value(&x) - 4.0).abs() < 1e-9, "x0 sub: {}", r.substitutions[0].value(&x));
    }

    #[test]
    fn cascading_elimination_of_two_free_variables() {
        // x0, x1 both free. Row0: x0 + x1 = 3. Row1: x0 = 2 (i.e. x0 appears
        // alone in row1 too). Eliminating x0 via row1 (larger |coeff|=1,
        // tie -> first found) then folds row0 down to a singleton in x1,
        // which itself has no remaining row appearance issue since x1 is
        // not free.
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0)]], 2);
        let b = vec![3.0, 4.0];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, f64::NEG_INFINITY];
        let ub = vec![f64::INFINITY, f64::INFINITY];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        // x0 eliminated via row1 (x0 = 4/2 = 2), folded into row0 =>
        // x1 = 3 - x0 = 1, so x1 also becomes eliminable next pass (row0
        // is now a singleton in x1).
        assert_eq!(r.substitutions.len(), 2);
        assert_eq!(r.a.nrows(), 0);
    }
}
