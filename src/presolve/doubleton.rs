//! DoubletonEquation (Achterberg et al., "Presolve Reductions in Mixed
//! Integer Programming", §4.5): an equality row with exactly two nonzero
//! variables, `a_i*x_i + a_k*x_k = rhs`, lets one of them be substituted
//! out in terms of the other — `x_i = (rhs - a_k*x_k) / a_i` — the same
//! elimination `colsingleton` performs, generalized to a variable that may
//! still appear in *other* rows (a true column singleton, by definition,
//! never does). Which variable is eliminated is a numerical-stability
//! choice, not a free one: solving for `x_i` divides every substituted
//! coefficient by `a_i`, so `a_i` should be the row's *larger*-magnitude
//! entry (the standard choice, e.g. as used by PaPILO's own
//! `DoubletonEquation` presolver) — eliminating the smaller one would
//! divide by the smaller number, amplifying rather than damping whatever
//! floating-point noise is already in `rhs`/`a_k`.
//!
//! Every row elsewhere that still references the eliminated variable
//! (`A`'s other rows and `G`'s, including `G`'s own single-variable box-
//! bound rows) is rewritten in place via the same substitution formula —
//! unlike `colsingleton`, which never needs this step because its
//! eliminated variable has no other appearances to rewrite. The box-bound
//! preservation this needs (deriving `lb_i <= x_i <= ub_i`'s equivalent
//! constraint on the surviving variable before `x_i`'s own bound rows are
//! folded away) is exactly `colsingleton`'s own derivation, reused
//! verbatim.

use crate::presolve::colsingleton::Substitution;
use crate::presolve::propagate;
use crate::sparse::{Csr, SparseAccum, csr_from_rows, csr_rows_pruned};

const TOL: f64 = 1e-9;

pub struct DoubletonResult {
    pub a: Csr,
    pub b: Vec<f64>,
    pub g: Csr,
    pub h: Vec<f64>,
    pub c: Vec<f64>,
    pub substitutions: Vec<Substitution>,
}

/// Rewrites `row`/`rhs` in place for every eliminated variable `row`
/// references (looked up via `by_var`, a `var -> index into subs` map),
/// merging duplicate column indices (a row can gain a term for a variable
/// it already had) via a scratch map. **Transitive**: substituting `j`
/// out can introduce a term for a variable that is *itself* eliminated by
/// a different row this same pass (a chain, e.g. `x0` kept in terms of
/// `x1`, `x1` in turn eliminated in terms of `x2`) — a single flat pass
/// over `row` would leave that freshly-introduced term unresolved, so any
/// newly-produced term is queued back through the same substitution check
/// rather than written straight to `merged`; `claimed`'s own guard against
/// a row using an already-claimed variable as *either* of its two terms
/// (see the pass below) rules out a cycle, so this always terminates. See
/// the module docs for the substitution algebra itself.
fn rewrite_row(accum: &mut SparseAccum, row: &[(usize, f64)], rhs: f64, subs: &[Substitution], by_var: &[Option<usize>]) -> (Vec<(usize, f64)>, f64) {
    // Fast path, bit-identical to the general one below: a row with
    // strictly increasing columns (so no duplicate to merge) that touches
    // no substituted variable comes out of the accumulator as itself,
    // minus entries at or below `TOL` — each `accum.add` is the first
    // write to its slot, so the stored value is `v` exactly.
    if row.windows(2).all(|w| w[0].0 < w[1].0) && row.iter().all(|&(j, _)| by_var[j].is_none()) {
        return (row.iter().copied().filter(|&(_, v)| v.abs() > TOL).collect(), rhs);
    }
    // The surviving terms land in the caller's shared sparse accumulator
    // rather than a `BTreeMap` built per rewritten row — see
    // `crate::sparse::SparseAccum`'s own docs. `take_sorted` emits in
    // ascending column order, so the rewritten row's own ordering (which
    // feeds later tie-breaks) is unchanged.
    accum.reset();
    let mut new_rhs = rhs;
    let mut queue: Vec<(usize, f64)> = row.to_vec();
    while let Some((j, v)) = queue.pop() {
        if let Some(idx) = by_var[j] {
            let sub = &subs[idx];
            new_rhs -= v * sub.rhs / sub.coeff;
            for &(k, term_v) in &sub.terms {
                queue.push((k, -v * term_v / sub.coeff));
            }
        } else {
            accum.add(j, v);
        }
    }
    (accum.take_sorted(TOL), new_rhs)
}

/// One non-cascading pass: candidate doubleton rows and which variable
/// each eliminates are all decided from `a`'s *input* shape — a variable
/// only claimed as "already eliminated" within this same pass (so two
/// doubleton rows never both try to eliminate it), not re-checked after
/// rewriting (mirrors `colsingleton`'s own single-pass scope).
pub fn eliminate_doubleton_equalities(n: usize, a: &Csr, b: &[f64], g: &Csr, h: &[f64], c: &[f64]) -> DoubletonResult {
    let a_rows: Vec<Vec<(usize, f64)>> = csr_rows_pruned(a);
    // One sparse accumulator for every `rewrite_row` call below.
    let mut accum = SparseAccum::new(n);

    let (lb, ub, real_g_rows, real_g_rhs) = propagate::extract_bounds(n, g, h);

    // `subs` in true discovery (row-iteration) order — required for
    // correct recovery later (a *different* round's substitution can
    // depend on this round's, and must be resolved after it; sorting by
    // variable index instead, as a `BTreeMap`-keyed collection would,
    // scrambles that relationship). `claimed` guards *both* of a
    // candidate row's variables, not just the one it would eliminate:
    // a row whose "keep" side already belongs to an earlier-claimed
    // variable (in this same pass) can't be treated as an independent
    // doubleton either — that would silently drop the earlier
    // elimination's own effect on this row (which needs a real
    // `rewrite_row` substitution, not to be treated as if the claimed
    // variable were still a live decision variable). Such a row is simply
    // deferred: left as a surviving row below, rewritten in terms of the
    // earlier substitution, and available again as a fresh candidate on
    // the *next* round.
    let mut subs: Vec<Substitution> = Vec::new();
    let mut by_var: Vec<Option<usize>> = vec![None; n];
    let mut claimed: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut eliminated_a_row = vec![false; a_rows.len()];

    for (i, row) in a_rows.iter().enumerate() {
        if row.len() != 2 {
            continue;
        }
        let (mut ti, mut tk) = (row[0], row[1]);
        if ti.1.abs() < tk.1.abs() {
            std::mem::swap(&mut ti, &mut tk);
        }
        let (var_elim, coeff_elim) = ti;
        let (var_keep, coeff_keep) = tk;
        if coeff_elim.abs() < TOL || claimed.contains(&var_elim) || claimed.contains(&var_keep) || var_elim == var_keep {
            continue;
        }
        let rhs = b[i];

        claimed.insert(var_elim);
        by_var[var_elim] = Some(subs.len());
        subs.push(Substitution { var: var_elim, terms: vec![(var_keep, coeff_keep)], rhs, coeff: coeff_elim });
        eliminated_a_row[i] = true;
    }

    // Preserve each eliminated variable's own box bounds as a `G` row on
    // its surviving partner — identical derivation to `colsingleton`'s
    // (see that module's docs for why skipping this silently unconstrains
    // the partner). Passed through `rewrite_row` just like every other
    // surviving row below: `var_keep` here can itself be a *different*
    // row's `var_elim` within this same pass (chained doubletons, e.g.
    // `x0` kept in terms of `x1`, `x1` in turn eliminated in terms of
    // `x2`) — skipping this rewrite would leave the bound-preserving row
    // pointing at `x1` after `x1` itself has zero real appearances left
    // anywhere else, silently dropping `x0`'s bound constraint from the
    // reduced problem instead of correctly chaining it onto `x2`.
    // A genuinely infinite `lb`/`ub` (a real free or one-sided-unbounded
    // source variable, not a finite sentinel) makes the corresponding
    // side's derived row vacuous (`r <= +inf`) — omitted rather than
    // emitted with an infinite `h`, same reasoning and arithmetic as
    // `colsingleton`'s own identical derivation (see that module's docs).
    let mut extra_g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut extra_h: Vec<f64> = Vec::new();
    for sub in &subs {
        let (var_keep, coeff_keep) = sub.terms[0];
        let a_lb = sub.coeff * lb[sub.var];
        let a_ub = sub.coeff * ub[sub.var];
        let lo = a_lb.min(a_ub);
        let hi = a_lb.max(a_ub);
        if lo.is_finite() {
            let (row1, rhs1) = rewrite_row(&mut accum, &[(var_keep, coeff_keep)], sub.rhs - lo, &subs, &by_var);
            extra_g_rows.push(row1);
            extra_h.push(rhs1);
        }
        if hi.is_finite() {
            let (row2, rhs2) = rewrite_row(&mut accum, &[(var_keep, -coeff_keep)], hi - sub.rhs, &subs, &by_var);
            extra_g_rows.push(row2);
            extra_h.push(rhs2);
        }
    }

    // Rewrite every surviving row (A's non-doubleton rows, G's real rows)
    // that references an eliminated variable, plus the objective.
    let mut new_a_rows = Vec::with_capacity(a_rows.len());
    let mut new_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if eliminated_a_row[i] {
            continue;
        }
        let (new_row, new_rhs) = rewrite_row(&mut accum, &row, b[i], &subs, &by_var);
        new_a_rows.push(new_row);
        new_b.push(new_rhs);
    }

    let mut new_g_rows = Vec::with_capacity(real_g_rows.len() + extra_g_rows.len());
    let mut new_h = Vec::with_capacity(real_g_rhs.len() + extra_h.len());
    for (row, rhs) in real_g_rows.into_iter().zip(real_g_rhs) {
        let (new_row, new_rhs) = rewrite_row(&mut accum, &row, rhs, &subs, &by_var);
        new_g_rows.push(new_row);
        new_h.push(new_rhs);
    }
    // Surviving (non-eliminated) variables' own box bounds, re-folded as
    // single-variable rows exactly as `build_a_g`/`propagate` do — `lb`/
    // `ub` themselves are untouched by this pass (only `subs`' own
    // variables lose their explicit bound rows, replaced by the
    // `extra_g_rows` derived above).
    for j in 0..n {
        if by_var[j].is_some() {
            continue;
        }
        if ub[j].is_finite() {
            new_g_rows.push(vec![(j, 1.0)]);
            new_h.push(ub[j]);
        }
        if lb[j].is_finite() {
            new_g_rows.push(vec![(j, -1.0)]);
            new_h.push(-lb[j]);
        }
    }
    new_g_rows.extend(extra_g_rows);
    new_h.extend(extra_h);

    let mut new_c = c.to_vec();
    for sub in &subs {
        let cj = new_c[sub.var];
        if cj != 0.0 {
            for &(k, v) in &sub.terms {
                new_c[k] -= cj * v / sub.coeff;
            }
            new_c[sub.var] = 0.0;
        }
    }

    DoubletonResult {
        a: csr_from_rows(&new_a_rows, n),
        b: new_b,
        g: csr_from_rows(&new_g_rows, n),
        h: new_h,
        c: new_c,
        substitutions: subs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_row_vec;

    #[test]
    fn eliminates_larger_coefficient_variable_and_rewrites_other_rows() {
        // Doubleton: 4*x0 + 2*x1 = 12  ->  eliminate x0 (|4|>|2|): x0 = (12-2*x1)/4 = 3 - 0.5*x1
        // Other A row referencing x0: x0 + x2 = 5  ->  after substitution: -0.5*x1 + x2 = 2
        let a = csr_from_rows(&[vec![(0, 4.0), (1, 2.0)], vec![(0, 1.0), (2, 1.0)]], 3);
        let b = vec![12.0, 5.0];
        // Bounds folded into g/h (build_a_g's convention): x0 in [0,10], x1 in [0,10], x2 in [0,10]
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
            ],
            3,
        );
        let h = vec![10.0, 0.0, 10.0, 0.0, 10.0, 0.0];
        let c = vec![0.0, 0.0, 0.0];

        let result = eliminate_doubleton_equalities(3, &a, &b, &g, &h, &c);
        assert_eq!(result.substitutions.len(), 1);
        let sub = &result.substitutions[0];
        assert_eq!(sub.var, 0);
        assert_eq!(sub.terms, vec![(1, 2.0)]);
        assert_eq!(sub.rhs, 12.0);
        assert_eq!(sub.coeff, 4.0);

        // The doubleton row itself is gone; the other A row survives, rewritten.
        assert_eq!(result.a.nrows(), 1);
        let row0 = csr_row_vec(&result.a, 0);
        assert!(row0.iter().any(|&(j, v)| j == 1 && (v - (-0.5)).abs() < 1e-9));
        assert!(row0.iter().any(|&(j, v)| j == 2 && (v - 1.0).abs() < 1e-9));
        assert!(!row0.iter().any(|&(j, _)| j == 0));
        assert!((result.b[0] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn recovers_eliminated_variable_value_within_its_own_bounds() {
        let a = csr_from_rows(&[vec![(0, 4.0), (1, 2.0)]], 2);
        let b = vec![12.0];
        let g = csr_from_rows(&[vec![(0, 1.0)], vec![(0, -1.0)], vec![(1, 1.0)], vec![(1, -1.0)]], 2);
        let h = vec![10.0, 0.0, 10.0, 0.0];
        let c = vec![0.0, 0.0];
        let result = eliminate_doubleton_equalities(2, &a, &b, &g, &h, &c);
        let sub = &result.substitutions[0];
        // x1 = 3 (a valid, in-bounds choice) -> x0 should recover to (12 - 2*3)/4 = 1.5
        let x = [0.0, 3.0];
        assert!((sub.value(&x) - 1.5).abs() < 1e-9);
    }
}
