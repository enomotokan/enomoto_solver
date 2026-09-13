//! Sparsify (HiGHS/PaPILO "Sparsify" presolver): rewrites a row `r`
//! (either an equality row of `A` or a real, multi-variable inequality row
//! of `G`) as `r - scale * eq` for some *other* equality row `eq` whose
//! entire support is contained in `r`'s (`S_eq subset-eq S_r`) —
//! guaranteed to remove at least one nonzero from `r` (the variable used
//! to derive `scale`) with **zero fill-in**, since every variable `eq`
//! touches is already present in `r`. This is the conservative half of
//! the general technique (PaPILO/HiGHS also allow a bounded amount of
//! fill-in when the net nonzero count still improves) — restricting to
//! the subset case trades away those opportunities for a substitution
//! that can never make anything *less* sparse, no fill-in accounting or
//! budget needed to prove it.
//!
//! Both rows still describe the exact same feasible region afterward:
//! `eq` itself is unchanged and still holds, so `r - scale*eq` is
//! satisfied by every point that satisfies both `r` and `eq`, and `r` can
//! be recovered from it by adding `scale*eq` back — a standard reversible
//! row operation, valid whether `r` is an equality or an inequality (only
//! its coefficients/RHS change; its own sense is untouched).
//!
//! Unlike `doubleton`/`colsingleton`, this never eliminates a variable or
//! a row outright — it only makes existing rows sparser, which can turn a
//! row into a fresh row singleton, or shrink a variable's own column
//! degree enough to make it eligible for `colsingleton`/`dualfix` on a
//! later pass (the same "unlocks a later stage" relationship
//! `run_extended`'s own module docs describe for its other passes).
//!
//! Candidate search, for each equality row `eq` (support `S_eq`, needs
//! `|S_eq| >= 2`): among `S_eq`'s own variables, the one with the fewest
//! *other* row appearances (`anchor`) bounds how many candidate target
//! rows need checking at all — every row that could possibly contain all
//! of `S_eq` must at least contain `anchor`. For each such candidate `r`
//! (skipping `eq` itself and anything shorter than `S_eq`, which can
//! never be a superset), `S_eq subset-eq S_r` is checked directly against
//! `r`'s own coefficients. The variable actually eliminated (`scale`'s
//! denominator) is `S_eq`'s own largest-magnitude entry, not necessarily
//! `anchor` — the same "eliminate the row's largest term" stability rule
//! `doubleton` uses (dividing by the smallest available coefficient
//! amplifies rounding noise), decoupled here from which variable happened
//! to be cheapest to search candidates on.
//!
//! **A row already rewritten as a target this call is never itself used
//! as a later pivot** — this is required for correctness, not just a
//! simplifying scope choice. An earlier version allowed it (reading every
//! pivot from an immutable start-of-call snapshot, on the reasoning that
//! a row's *original* content is an equally valid fact about the system
//! regardless of what its own stored form has since become): confirmed
//! on Netlib `scorpion` to silently corrupt the system when two rows
//! reference each other this way in the same pass (row 0 used to
//! sparsify row 1, then row 1's *original* content — while row 1 itself
//! had just been overwritten — used to sparsify row 0 right back).
//! Each individual substitution is a locally valid row operation, but
//! replacing a *pair* (or longer chain) of rows with independently-
//! computed linear combinations of their originals is only guaranteed
//! equivalent to the original pair if the combined transformation is
//! invertible — true for an isolated pair often enough to not show up
//! immediately, but not guaranteed once many such substitutions chain
//! together across a whole pass, where it measurably was not. Forbidding
//! a just-modified row from being read as a pivot keeps every pivot's
//! own stored form and the fact used to derive other rows identical,
//! side-stepping the question entirely: a target is always rewritten
//! against a pivot whose own row is returned completely unchanged.
//!
//! Single-pass, non-cascading otherwise (mirrors `doubleton`/
//! `colsingleton`'s own scope): a target row is rewritten at most once
//! per call. A chain of opportunities this pass reveals (a target
//! sparsified here newly becoming a valid pivot, or a newly-shrunk row
//! unlocking a different target) is picked up by `run_extended`'s outer
//! fixpoint loop calling this again, not resolved here.
//!
//! **Implemented, unit-tested (including a regression test for the
//! mutual-reference bug above), and measured against the full Netlib
//! set — then left unintegrated (kept here, tested, but never called
//! from [`crate::presolve::run_extended`]).** Wired in once per outer
//! round, right before `colsingleton`: with the correctness bug fixed,
//! zero objective mismatches across all 73 Netlib instances — but total
//! `ours` time went from 3.68s to 7.30s (a ~2x aggregate regression),
//! concentrated almost entirely on a handful of instances already known
//! in this crate's own history to be unusually sensitive to *any* change
//! in matrix shape: `degen3` (+390%), `25fv47` (+315%), `wood1p`
//! (+127%), `cycle` (+95%) — `iters` on `degen3` alone went from 2,422 to
//! 11,676 with an *identical, still-correct* final objective, the same
//! "reshaping the matrix changes chuzc/DSE tie-breaking, which cascades
//! into a completely different (and on a degenerate instance, potentially
//! far longer) pivot sequence" pattern this session already hit
//! repeatedly for other structural presolve changes (the
//! Dulmage-Mendelsohn block-triangularization attempt documented in
//! `simplex::lu`, connected-component splitting's own "at least 2 real
//! components" gate in `simplex.rs`). A handful of small instances did
//! improve (`bnl1` -21%, `brandy` -29%), but nowhere near enough to
//! offset the losses. No cheap pre-gate (route only instances likely to
//! benefit) was tried before reverting — see this doc comment's own
//! commit for the full numbers if revisiting.

use crate::presolve::propagate;
use crate::sparse::{csr_from_rows, Csr};
use std::collections::BTreeMap;

const TOL: f64 = 1e-9;

pub struct SparsifyResult {
    pub a: Csr,
    pub b: Vec<f64>,
    pub g: Csr,
    pub h: Vec<f64>,
    /// How many rows (`A`'s and `G`'s combined) were rewritten this call —
    /// `0` means this pass found nothing, letting `run_extended`'s
    /// fixpoint loop stop repeating it.
    pub n_rows_changed: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Src {
    A,
    G,
}

pub fn sparsify(n: usize, a: &Csr, b: &[f64], g: &Csr, h: &[f64]) -> SparsifyResult {
    let ar = a.as_ref();
    let mut a_rows: Vec<Vec<(usize, f64)>> = (0..ar.nrows())
        .map(|i| ar.col_indices_of_row(i).zip(ar.values_of_row(i)).map(|(j, &v)| (j, v)).filter(|&(_, v)| v != 0.0).collect())
        .collect();
    let mut b: Vec<f64> = b.to_vec();

    let (lb, ub, mut real_g_rows, mut real_g_rhs) = propagate::extract_bounds(n, g, h);

    // Column -> (source, row index) for every currently-eligible *target*
    // row (`A`'s own rows plus `G`'s real, multi-variable rows) — box-
    // bound rows are never touched, as either pivot or target: they're
    // already minimal. Built once, from the same pre-rewrite state.
    let mut col_to_rows: Vec<Vec<(Src, usize)>> = vec![Vec::new(); n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, _) in row {
            col_to_rows[j].push((Src::A, i));
        }
    }
    for (i, row) in real_g_rows.iter().enumerate() {
        for &(j, _) in row {
            col_to_rows[j].push((Src::G, i));
        }
    }

    let mut changed_a = vec![false; a_rows.len()];
    let mut changed_g = vec![false; real_g_rows.len()];
    let mut n_rows_changed = 0usize;

    for eq_idx in 0..a_rows.len() {
        // A row already rewritten as someone else's target this call is
        // never used as a pivot — see the module docs for why this is
        // required for correctness, not just scope. `b[eq_idx]` is read
        // fresh here (not from a start-of-call snapshot) for the same
        // reason: by the time this row is used as a pivot, `changed_a`
        // guarantees it hasn't been touched yet this call, so its current
        // stored content and rhs *are* its original, untouched form.
        if changed_a[eq_idx] {
            continue;
        }
        let eq_row = a_rows[eq_idx].clone();
        if eq_row.len() < 2 {
            continue;
        }
        let anchor = eq_row.iter().map(|&(j, _)| j).min_by_key(|&j| col_to_rows[j].len()).unwrap();
        // Elimination target within `eq` itself: largest-magnitude entry.
        let (elim_var, elim_coeff) = *eq_row.iter().max_by(|x, y| x.1.abs().total_cmp(&y.1.abs())).unwrap();
        if elim_coeff.abs() < TOL {
            continue;
        }
        let eq_map: BTreeMap<usize, f64> = eq_row.iter().copied().collect();
        let eq_rhs = b[eq_idx];

        for &(src, idx) in &col_to_rows[anchor] {
            if src == Src::A && idx == eq_idx {
                continue;
            }
            let already_changed = match src {
                Src::A => changed_a[idx],
                Src::G => changed_g[idx],
            };
            if already_changed {
                continue;
            }
            let target_row = match src {
                Src::A => &a_rows[idx],
                Src::G => &real_g_rows[idx],
            };
            if target_row.len() < eq_row.len() {
                // Can never be a superset of `S_eq` — a cheap pre-filter
                // ahead of the O(|eq_row|) subset scan below.
                continue;
            }
            let mut new_row: BTreeMap<usize, f64> = target_row.iter().copied().collect();
            if !eq_map.keys().all(|k| new_row.contains_key(k)) {
                continue;
            }
            let target_elim_coeff = *new_row.get(&elim_var).expect("elim_var already verified to be in target's support");
            let scale = target_elim_coeff / elim_coeff;
            if scale == 0.0 {
                continue;
            }
            for (&j, &v) in &eq_map {
                *new_row.entry(j).or_insert(0.0) -= scale * v;
            }
            // Only `elim_var` is *proven* to cancel exactly (`scale` was
            // chosen specifically to zero it) — any other entry's
            // subtraction result, however small, is the mathematically
            // correct new coefficient, not noise, and must be kept as-is
            // rather than dropped by some absolute tolerance: a target
            // row's other shared coefficient can land close to (but not
            // at) zero by sheer coincidence without being a true
            // cancellation, and silently discarding it would corrupt the
            // row.
            new_row.remove(&elim_var);
            new_row.retain(|_, v| *v != 0.0);
            let new_rhs = match src {
                Src::A => b[idx] - scale * eq_rhs,
                Src::G => real_g_rhs[idx] - scale * eq_rhs,
            };
            let new_row_vec: Vec<(usize, f64)> = new_row.into_iter().collect();
            match src {
                Src::A => {
                    a_rows[idx] = new_row_vec;
                    b[idx] = new_rhs;
                    changed_a[idx] = true;
                }
                Src::G => {
                    real_g_rows[idx] = new_row_vec;
                    real_g_rhs[idx] = new_rhs;
                    changed_g[idx] = true;
                }
            }
            n_rows_changed += 1;
        }
    }

    let (g, h) = propagate::rebuild_g(n, real_g_rows, real_g_rhs, &lb, &ub);
    SparsifyResult { a: csr_from_rows(&a_rows, n), b, g, h, n_rows_changed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparsifies_a_superset_inequality_row_with_zero_fill_in() {
        // eq: x0 + x1 = 5 (support {0,1}).
        // target (G, real row): x0 + x1 + x2 <= 10 (support {0,1,2}, a
        // strict superset) -> after subtracting 1*eq: x2 <= 5.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 3);
        let b = vec![5.0];
        let g = csr_from_rows(
            &[
                vec![(0, 1.0), (1, 1.0), (2, 1.0)], // target
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
            ],
            3,
        );
        let h = vec![10.0, 10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(3, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 1);

        // The sparsified row (`x2 <= 5`) has exactly one variable, so
        // `extract_bounds` correctly folds it straight into `ub[2]` as a
        // tighter bound rather than keeping it as a "real" multi-variable
        // row — a free extra reduction, not a bug in this test.
        let (_, ub, real_rows, _) = propagate::extract_bounds(3, &result.g, &result.h);
        assert!(real_rows.is_empty(), "the fully-sparsified row collapsed into a bound, not a real row");
        assert!((ub[2] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn sparsifies_a_superset_equality_row() {
        // eq: x0 + x1 = 5. target (A): 2*x0 + 2*x1 + 3*x2 = 20
        // -> subtract 2*eq: 3*x2 = 10.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0), (2, 3.0)]], 3);
        let b = vec![5.0, 20.0];
        let g = csr_from_rows(
            &[vec![(0, 1.0)], vec![(0, -1.0)], vec![(1, 1.0)], vec![(1, -1.0)], vec![(2, 1.0)], vec![(2, -1.0)]],
            3,
        );
        let h = vec![10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(3, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 1);

        let ar = result.a.as_ref();
        let row1: Vec<(usize, f64)> = ar.col_indices_of_row(1).zip(ar.values_of_row(1)).map(|(j, &v)| (j, v)).collect();
        assert_eq!(row1, vec![(2, 3.0)]);
        assert!((result.b[1] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn leaves_a_non_superset_row_untouched() {
        // eq: x0 + x1 = 5. target: x0 + x2 <= 10 -- doesn't contain x1,
        // so `S_eq` isn't a subset of the target's support; nothing to do.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 3);
        let b = vec![5.0];
        let g = csr_from_rows(
            &[
                vec![(0, 1.0), (2, 1.0)],
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
            ],
            3,
        );
        let h = vec![10.0, 10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(3, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 0);
    }

    #[test]
    fn a_row_already_used_as_a_target_is_never_used_as_a_later_pivot() {
        // Three equality rows sharing structure such that, without the
        // "never re-pivot an already-rewritten row" guard, row 0 would
        // sparsify row 1, then row 1's *original* content (while row 1
        // itself had just been overwritten) would sparsify row 0 right
        // back — silently corrupting the pair (confirmed on Netlib
        // `scorpion`; see the module docs). Regression test for that.
        //
        // eq0: x0 + x1 = 3            (support {0,1})
        // eq1: x0 + x1 + x2 + x3 = 10 (support {0,1,2,3}, superset of eq0)
        // After eq0 sparsifies eq1 (subtract 1*eq0): eq1' = x2 + x3 = 7.
        // eq1's *original* support ({0,1,2,3}) is not a subset of eq0's
        // ({0,1}), so eq1 (original) could never legitimately sparsify
        // eq0 anyway -- eq0 must survive this call completely unchanged.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)]], 4);
        let b = vec![3.0, 10.0];
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
                vec![(3, 1.0)],
                vec![(3, -1.0)],
            ],
            4,
        );
        let h = vec![10.0, 0.0, 10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(4, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 1);

        let ar = result.a.as_ref();
        let row0: Vec<(usize, f64)> = ar.col_indices_of_row(0).zip(ar.values_of_row(0)).map(|(j, &v)| (j, v)).collect();
        assert_eq!(row0, vec![(0, 1.0), (1, 1.0)], "eq0 must survive this call completely unchanged");
        assert!((result.b[0] - 3.0).abs() < 1e-9);

        let row1: Vec<(usize, f64)> = ar.col_indices_of_row(1).zip(ar.values_of_row(1)).map(|(j, &v)| (j, v)).collect();
        assert_eq!(row1, vec![(2, 1.0), (3, 1.0)]);
        assert!((result.b[1] - 7.0).abs() < 1e-9);
    }
}
