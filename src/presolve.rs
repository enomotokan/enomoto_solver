//! Shared presolve pipeline: **the same** scaling, redundant-equality
//! removal and inequality-propagation passes feed both `simplex.rs`
//! (active) and `interior_point.rs` (inactive) — there is exactly one
//! implementation of each pass, run identically by both engines, not two
//! parallel copies that could drift apart.
//!
//! ## Shape
//!
//! Both engines want the same `A x = b`, `G x <= h` shape (variable bounds
//! folded into `G` as single-variable rows — see [`build_a_g`]), so the
//! pipeline itself is expressed purely in terms of that shape and knows
//! nothing about either engine's own internal representation
//! (`interior_point`'s KKT blocks, `simplex`'s `StdForm`):
//!
//!   1. [`scaling::compute`] + [`scaling::apply`]: modified Ruiz
//!      equilibration, run once on the original (unscaled) `A`/`G`/`c`.
//!   2. [`redundancy::reduce_equalities`] + [`redundancy::reduce_inequalities`]:
//!      drops duplicate/linearly-dependent rows from the scaled `(A, b)`,
//!      and duplicate/dominated rows from the scaled `(G, h)`.
//!   3. [`dualfix::fix_dominated_variables`]: fixes any variable whose
//!      objective cost prefers a direction no real row resists, straight
//!      off the coefficient matrix — no simplex iteration needed. Its
//!      fixes are folded back into `G`/`h` via [`propagate::rebuild_g`]
//!      before the next step, so propagation (below) can chain off newly
//!      fixed bounds too.
//!   4. [`propagate::propagate`], run `passes` times: activity-bound
//!      constraint propagation over the scaled `(G, h)`, tightening
//!      variable bounds and dropping/detecting redundant/infeasible rows.
//!
//! [`run`] packages exactly this sequence — steps 1, 2's equality half and
//! 4 are the same ones `interior_point.rs` ran inline before this
//! pipeline was extracted — so both callers get identical behavior from
//! one call site each.
//!
//! [`colsingleton::eliminate_singleton_equalities`] is deliberately *not*
//! part of this pipeline: it runs once, directly in `simplex.rs`, on the
//! unscaled `(A, b, c)` straight out of [`build_a_g`], before `run` is
//! ever called — see that module's docs for why it needs to stay in
//! original (pre-scaling) units.
//!
//! `simplex.rs` additionally calls [`propagate::extract_bounds`] on the
//! *final* `G`/`h` this pipeline returns, to pull the (possibly tightened)
//! box bounds back out as `StdForm`'s explicit `lb`/`ub` rather than
//! leaving them as constraint rows with their own slack — a variable's
//! bound is represented as a bound, in both engines, not as an extra
//! artificial/slack variable. The genuinely remaining multi-variable rows
//! become ordinary `Eq`/`Le` rows, handled by whatever feasibility
//! mechanism each engine already has (interior-point's central-path
//! Newton iteration; the simplex method's own phase 1 / dual-feasible
//! crash) — this pipeline introduces no new variable of its own to either
//! engine's standard form.

pub mod colsingleton;
pub mod dualfix;
pub mod propagate;
pub mod redundancy;
pub mod scaling;

use crate::sparse::{csr_from_rows, Csr};
use crate::types::{ConstraintRow, RowSense, VariableData};
use scaling::Scaling;

/// Builds `A x = b`, `G x <= h` (bounds folded into `G` as single-variable
/// rows — `ub` as `(j, 1.0)`/`h=ub`, `lb` as `(j, -1.0)`/`h=-lb`, always
/// present since every variable has two finite bounds — see the
/// bounded-variable invariant documented in `simplex.rs`'s module docs)
/// directly from the model's variables/constraints. Shared by
/// `interior_point::qp::build` (which adds its own `c` from
/// `obj_coeffs_for_min`) and `simplex.rs`'s presolve entry point.
pub fn build_a_g(variables: &[VariableData], constraints: &[ConstraintRow]) -> (Csr, Vec<f64>, Csr, Vec<f64>) {
    let n = variables.len();

    let mut a_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut b: Vec<f64> = Vec::new();
    let mut g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut h: Vec<f64> = Vec::new();

    for row in constraints {
        let terms: Vec<(usize, f64)> = row.expr.coeffs.iter().map(|(&j, &v)| (j, v)).collect();
        let rhs = row.rhs - row.expr.constant;
        match row.sense {
            RowSense::Eq => {
                a_rows.push(terms);
                b.push(rhs);
            }
            RowSense::Le => {
                g_rows.push(terms);
                h.push(rhs);
            }
            RowSense::Ge => {
                g_rows.push(terms.into_iter().map(|(j, v)| (j, -v)).collect());
                h.push(-rhs);
            }
        }
    }

    for (j, v) in variables.iter().enumerate() {
        g_rows.push(vec![(j, 1.0)]);
        h.push(v.ub);
        g_rows.push(vec![(j, -1.0)]);
        h.push(-v.lb);
    }

    let a = csr_from_rows(&a_rows, n);
    let g = csr_from_rows(&g_rows, n);
    (a, b, g, h)
}

/// The result of running the full shared presolve pipeline once: the
/// scaled-and-reduced problem data, the [`Scaling`] needed to map a
/// solution back to the original variables (via [`scaling::unscale_x`]),
/// and an `infeasible` flag `propagate` can raise directly (an activity
/// bound proving a row can never be satisfied) without either engine
/// having to run its own solve loop first.
pub struct PresolveResult {
    pub scaling: Scaling,
    pub a: Csr,
    pub b: Vec<f64>,
    pub g: Csr,
    pub h: Vec<f64>,
    pub c: Vec<f64>,
    pub infeasible: bool,
}

/// Runs the shared pipeline described in the module docs: scale, drop
/// redundant equality/inequality rows, fix any dominated variables
/// outright, then propagate the result for `prop_passes` rounds.
/// `ruiz_iters` and `prop_passes` are left as parameters (rather than
/// shared constants) so each engine keeps its own tuning — both currently
/// pass the same values (`10` and `2`) `interior_point.rs` used before
/// this pipeline was extracted.
pub fn run(n: usize, a: &Csr, b: &[f64], g: &Csr, h: &[f64], c: &[f64], ruiz_iters: usize, prop_passes: usize) -> PresolveResult {
    let sc = scaling::compute(n, a, g, c, ruiz_iters);
    let (a, g, b, h, c) = scaling::apply(&sc, a, g, b, h, c);

    let (a, b) = redundancy::reduce_equalities(&a, &b, n);
    let (g, h) = redundancy::reduce_inequalities(&g, &h, n);

    let (lb0, ub0, real_rows, real_rhs) = propagate::extract_bounds(n, &g, &h);
    // `dualfix` trusts `lb0`/`ub0` to already be self-consistent (see
    // `bounds_inconsistent`'s own docs) — an inconsistency here is a real
    // infeasibility that must be reported directly, not smoothed over by
    // "fixing" a variable to one of its two contradictory bounds.
    if propagate::bounds_inconsistent(n, &lb0, &ub0) {
        return PresolveResult { scaling: sc, a, b, g: csr_from_rows(&[], n), h: Vec::new(), c, infeasible: true };
    }
    let fixes = dualfix::fix_dominated_variables(n, &a, &real_rows, &c, &lb0, &ub0);
    let (g, h) = if fixes.is_empty() {
        (g, h)
    } else {
        let mut lb0 = lb0;
        let mut ub0 = ub0;
        for &(j, value) in &fixes {
            lb0[j] = value;
            ub0[j] = value;
        }
        propagate::rebuild_g(n, real_rows, real_rhs, &lb0, &ub0)
    };

    let prop = propagate::propagate(n, &g, &h, prop_passes);

    PresolveResult { scaling: sc, a, b, g: prop.g, h: prop.h, c, infeasible: prop.infeasible }
}
