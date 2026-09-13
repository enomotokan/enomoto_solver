//! ColSingleton (Achterberg et al. 2019; PaPILO's `ColSingleton`
//! presolver): a variable whose *column* has exactly one nonzero entry
//! across every row — appears in exactly one constraint, full stop — can,
//! when that one row is an equality, always be substituted out: solve the
//! row for the variable in terms of the row's other variables, fold its
//! objective cost into theirs, and drop both the variable and the row
//! from what the solver actually has to work with. The eliminated
//! variable's true value is recovered afterward by plugging the other
//! (now-solved) variables from that same row back into the formula.
//!
//! "Appears in exactly one row" only counts *real* rows — a variable's
//! own box-bound rows in `G` don't count any more than they do for
//! `dualfix` (see that module's docs); this module additionally only
//! looks at `A` for candidates, since only an equality row's singleton
//! case is a *substitution* (the inequality case needs a sign-based case
//! analysis of whether the row is guaranteed to bind, and is deferred).
//!
//! Dropping the row is only sound if the eliminated variable's own box
//! bounds are preserved *somewhere*: `x_j = (rhs - r) / coeff` (`r` = the
//! row's other terms) must still satisfy `lb_j <= x_j <= ub_j`, and
//! nothing else in the reduced problem enforces that once the row and the
//! variable's own place in the objective are both gone. So every
//! elimination here also emits up to two new `<=` rows on `r` alone —
//! derived by solving `lb_j <= (rhs - r)/coeff <= ub_j` for `r` — that the
//! caller must fold into `G`/`h`. Skipping this (the first version of
//! this module did) silently drops the eliminated variable's bounds: e.g.
//! `x0 + x1 = 5` with only `x0` eliminated left `x1` completely
//! unconstrained above, since removing the row was the *only* place
//! `x1`'s upper reach had been limited.
//!
//! A genuinely infinite `lb_j`/`ub_j` — a real free (or one-sided) source
//! variable, not a finite sentinel — makes the corresponding derived row
//! vacuous (`r <= +inf` constrains nothing) rather than merely very wide,
//! so it is omitted outright instead of emitted with an infinite `h`: the
//! classic "free column singleton" case (both bounds infinite) then costs
//! *zero* replacement rows, letting a chain of these cascade through a
//! network-shaped equality system the same way HiGHS's own presolve does.

use crate::presolve::propagate;
use crate::sparse::{csr_from_rows, Csr};

const TOL: f64 = 1e-9;
/// Minimum `|coeff| / max|row|` for a column singleton to be substituted
/// out — see the guard in `eliminate_singleton_equalities`.
const SUBSTITUTION_PIVOT_RATIO: f64 = 1e-2;

/// `x[var] = (rhs - sum(terms[k].1 * x[terms[k].0])) / coeff`, using the
/// *other* variables' already-solved values.
pub struct Substitution {
    pub var: usize,
    pub terms: Vec<(usize, f64)>,
    pub rhs: f64,
    pub coeff: f64,
}

impl Substitution {
    /// Recovers `x[self.var]` from the other variables' solved values in
    /// `x` (which must already hold correct values at every index this
    /// substitution's `terms` reference — guaranteed by construction,
    /// since a substituted variable's own row is never itself a term of
    /// another substitution; see `eliminate_singleton_equalities`'s docs).
    pub fn value(&self, x: &[f64]) -> f64 {
        let mut rhs = self.rhs;
        for &(k, v) in &self.terms {
            rhs -= v * x[k];
        }
        rhs / self.coeff
    }
}

pub struct EliminationResult {
    pub a: Csr,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    /// New `<=` rows the caller must append to `G`/`h`: each eliminated
    /// variable's own box bounds, translated onto its substitution's
    /// remaining variables (see the module docs for why this is required,
    /// not optional).
    pub extra_g_rows: Vec<Vec<(usize, f64)>>,
    pub extra_h: Vec<f64>,
    pub substitutions: Vec<Substitution>,
}

/// One non-cascading pass: appearance counts are computed once from the
/// input and never updated as eliminations are found, so a variable that
/// becomes a singleton only *after* another elimination removes its other
/// occurrence isn't caught here (a later call, given this pass's own
/// output, would catch it).
pub fn eliminate_singleton_equalities(n: usize, a: &Csr, b: &[f64], g: &Csr, h: &[f64], c: &[f64]) -> EliminationResult {
    let ar = a.as_ref();
    let a_rows: Vec<Vec<(usize, f64)>> = (0..ar.nrows())
        .map(|i| ar.col_indices_of_row(i).zip(ar.values_of_row(i)).map(|(j, &v)| (j, v)).collect())
        .collect();

    // `lb`/`ub` for the box-bound part; `real_g_rows` for the "does this
    // column appear anywhere in G besides its own bound rows" part — both
    // straight from the same extraction `propagate` itself uses.
    let (lb, ub, real_g_rows, _real_g_rhs) = propagate::extract_bounds(n, g, h);

    // Total appearances across every *real* row (A's rows, plus G's
    // multi-variable rows — G's own single-variable rows are box bounds,
    // not independent appearances, same exclusion `dualfix` applies).
    let mut appearances = vec![0usize; n];
    let mut owning_a_row = vec![None; n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                appearances[j] += 1;
                owning_a_row[j] = Some(i);
            }
        }
    }
    for row in &real_g_rows {
        for &(j, v) in row {
            if v != 0.0 {
                appearances[j] += 1;
            }
        }
    }

    let mut eliminated_rows = vec![false; a_rows.len()];
    let mut substitutions = Vec::new();
    let mut extra_g_rows = Vec::new();
    let mut extra_h = Vec::new();
    let mut new_c = c.to_vec();

    for j in 0..n {
        if appearances[j] != 1 {
            continue;
        }
        let Some(i) = owning_a_row[j] else { continue }; // its one appearance is in G, not A
        if eliminated_rows[i] {
            // This row already substituted a *different* singleton column
            // of its own (two singleton columns can share one row, e.g.
            // `x5 + x7 = 3` with neither appearing anywhere else) — only
            // one variable per row can be solved for; `j` remains a real
            // decision variable, referenced as a term in that other
            // substitution instead.
            continue;
        }
        let row = &a_rows[i];
        let coeff = row.iter().find(|&&(k, _)| k == j).unwrap().1;
        if coeff.abs() < TOL {
            continue;
        }
        // Markowitz-style pivot guard (PaPILO applies the same kind of
        // relative threshold before any substitution): solving the row for
        // `x_j` divides every other coefficient by `coeff`, so a `coeff`
        // that is tiny *relative to its own row* amplifies whatever
        // residual the reduced problem is later solved to (`~1e-7`) by
        // `max|row| / |coeff|` when `x_j` is recovered — measured on Netlib
        // `modszk1` as ~1e-6 violations of the eliminated rows and an
        // objective slightly *below* the true optimum. Such a column is
        // simply left in the problem.
        let row_max = row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < SUBSTITUTION_PIVOT_RATIO * row_max {
            continue;
        }
        let terms: Vec<(usize, f64)> = row.iter().filter(|&&(k, _)| k != j).cloned().collect();
        let rhs = b[i];

        let cj = new_c[j];
        if cj != 0.0 {
            for &(k, a_ik) in &terms {
                new_c[k] -= cj * a_ik / coeff;
            }
            new_c[j] = 0.0;
        }

        // x_j's own box bounds, translated onto `r = sum(terms)`:
        // lb_j <= (rhs - r)/coeff <= ub_j  <=>  rhs-hi <= r <= rhs-lo,
        // where lo/hi = min/max(coeff*lb_j, coeff*ub_j) handles either
        // sign of `coeff` uniformly. A side of `[lb_j, ub_j]` that was
        // genuinely infinite carries through this arithmetic to an
        // infinite `lo`/`hi` (finite `coeff` times `+/-inf` is exactly
        // `+/-inf`, correctly signed) — the corresponding row would then
        // be `r <= +inf`, true unconditionally, so it is skipped rather
        // than emitted with that infinite `h` (see the module docs' "free
        // column singleton" case above for why this is exactly the
        // reduction that matters).
        let a_lb = coeff * lb[j];
        let a_ub = coeff * ub[j];
        let lo = a_lb.min(a_ub);
        let hi = a_lb.max(a_ub);
        if lo.is_finite() {
            extra_g_rows.push(terms.clone());
            extra_h.push(rhs - lo);
        }
        if hi.is_finite() {
            extra_g_rows.push(terms.iter().map(|&(k, v)| (k, -v)).collect());
            extra_h.push(hi - rhs);
        }

        substitutions.push(Substitution { var: j, terms, rhs, coeff });
        eliminated_rows[i] = true;
    }

    let mut new_a_rows = Vec::with_capacity(a_rows.len());
    let mut new_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !eliminated_rows[i] {
            new_a_rows.push(row);
            new_b.push(b[i]);
        }
    }

    EliminationResult {
        a: csr_from_rows(&new_a_rows, n),
        b: new_b,
        c: new_c,
        extra_g_rows,
        extra_h,
        substitutions,
    }
}
