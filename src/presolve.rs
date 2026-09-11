//! Shared presolve pipeline: **the same** scaling, redundant-equality
//! removal, bound-propagation, and (row/column-singleton, doubleton,
//! dual-fixing) elimination passes feed both `simplex.rs` and
//! `interior_point.rs` — there is exactly one implementation of each pass,
//! run identically by both engines through the single [`run_extended`]
//! entry point, not two parallel copies (or two different pipelines) that
//! could drift apart.
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
//!   3. [`run_extended`]'s own round loop: [`propagate::propagate`]
//!      (activity-bound constraint propagation, tightening variable bounds
//!      and dropping/detecting redundant/infeasible rows) →
//!      [`dualfix::fix_dominated_variables`] (fixes any variable whose
//!      objective cost prefers a direction no real row resists, straight
//!      off the coefficient matrix) → [`rowsingleton`], [`doubleton`], then
//!      an *inner* fixpoint of [`rowsingleton`] <-> [`colsingleton`] alone
//!      (up to `inner_rounds` times, without repaying `propagate`/
//!      `dualfix`'s own cost — colsingleton eliminating a variable can turn
//!      a row rowsingleton had no reason to touch into a fresh row
//!      singleton, and vice versa, so this pair alone can have more to
//!      find after its own first pass; `doubleton` itself isn't part of
//!      this inner repetition — measured to be a net *regression* when it
//!      was), the whole thing repeated for up to `rounds` outer passes
//!      since a bound `propagate` tightens can unlock a `dualfix`/
//!      singleton/doubleton reduction the previous outer pass couldn't yet
//!      see. `doubleton` itself only runs during the first
//!      `doubleton_rounds` outer passes (once per pass) — measured to
//!      recover almost all of an eligible instance's doubleton
//!      opportunities within the first couple of rounds, so spending its
//!      own full-matrix scan on every one of up to `rounds` rounds
//!      regardless of whether anything new remains was a net loss on
//!      several instances. Both loops stop early, before their own cap,
//!      once a pass changes neither row count nor (for the outer loop) any
//!      bound — a fixpoint: every stage here is a deterministic function
//!      of exactly that state, so a pass that changes nothing leaves
//!      nothing for a further pass to find either.
//!
//! [`run_extended`] packages exactly this sequence into the one call site
//! both `simplex.rs` and `interior_point.rs` use, and returns every
//! eliminated variable's [`colsingleton::Substitution`] for the caller to
//! recover after solving (`simplex.rs`'s `unscale_result`,
//! `interior_point.rs`'s `unscale_with_substitutions` — same reverse-order
//! recovery, one implementation of the *technique*, two small call-site
//! adapters). An earlier version of this pipeline kept a second,
//! elimination-free entry point (`run`) that `interior_point.rs` used
//! instead, back when it had no mechanism to recover an eliminated
//! variable's value — removed once that mechanism was added, so every
//! technique ported into this pipeline (present or future) now benefits
//! both engines without a per-technique decision about which pipeline it
//! belongs in.
//!
//! `simplex.rs` builds `StdForm`'s explicit `lb`/`ub` (rather than leaving
//! bounds as constraint rows with their own slack — a variable's bound is
//! represented as a bound, in both engines, not as an extra artificial/
//! slack variable) straight from [`ExtendedPresolveResult`]'s own `lb`/
//! `ub`/`real_rows`/`real_rhs` fields — the same split [`propagate::propagate`]
//! already computed internally, exposed here instead of `simplex.rs`
//! re-deriving it with its own [`propagate::extract_bounds`] call on the
//! just-rebuilt `g`/`h`. `interior_point.rs`, by contrast, wants bounds
//! folded into `G` (its central-path Newton iteration has no separate
//! bound-handling machinery), so it reads [`ExtendedPresolveResult`]'s
//! `g`/`h` fields instead — both are the same [`propagate::propagate`]
//! final state, just exposed in whichever shape each caller wants. The
//! genuinely remaining multi-variable rows (`real_rows`/`real_rhs`) become
//! ordinary `Eq`/`Le` rows, handled by whatever feasibility mechanism each
//! engine already has (interior-point's central-path Newton iteration; the
//! simplex method's own phase 1 / dual-feasible crash) — this pipeline
//! introduces no new variable of its own to either engine's standard form.

pub mod colsingleton;
pub mod doubleton;
pub mod dualfix;
pub mod propagate;
pub mod redundancy;
pub mod rowsingleton;
pub mod scaling;
pub mod smallcoeff;

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
/// an `infeasible` flag `propagate` can raise directly (an activity bound
/// proving a row can never be satisfied) without either engine having to
/// run its own solve loop first, and every variable
/// [`doubleton::eliminate_doubleton_equalities`]/
/// [`colsingleton::eliminate_singleton_equalities`] substituted out along
/// the way — the caller must recover each one's true value via
/// [`colsingleton::Substitution::value`] *before* unscaling (these
/// coefficients are in [`run_extended`]'s scaled space, unlike
/// `colsingleton`'s old standalone, pre-scaling call site), and in
/// **reverse** discovery order: a later round's substitution can
/// reference a variable an *earlier* round already substituted out (that
/// variable dropped to zero remaining appearances then, so a later round
/// is the only one that could newly treat it as eliminable in turn),
/// never the other way around, so resolving latest-first is what
/// guarantees every `value()` call only ever reads already-known inputs.
///
/// `g`/`h` fold bounds back in as single-variable rows (what
/// `interior_point.rs` wants — including, for an eliminated variable, its
/// pinned-to-a-point `[0,0]` box-bound row, since `run_extended` already
/// applies that fix before deriving `g`/`h`); `lb`/`ub`/`real_rows`/
/// `real_rhs` are the same information already split apart (what
/// `simplex.rs` wants) — both are `propagate::propagate`'s own final
/// state, carried through here so callers needing either form never have
/// to re-derive it with a second `propagate::extract_bounds` call.
pub struct ExtendedPresolveResult {
    pub scaling: Scaling,
    pub a: Csr,
    pub b: Vec<f64>,
    pub g: Csr,
    pub h: Vec<f64>,
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    pub real_rows: Vec<Vec<(usize, f64)>>,
    pub real_rhs: Vec<f64>,
    pub c: Vec<f64>,
    pub infeasible: bool,
    pub substitutions: Vec<colsingleton::Substitution>,
}

fn extended_infeasible(sc: Scaling, a: Csr, b: Vec<f64>, c: Vec<f64>, n: usize) -> ExtendedPresolveResult {
    ExtendedPresolveResult {
        scaling: sc,
        a,
        b,
        g: csr_from_rows(&[], n),
        h: Vec::new(),
        lb: Vec::new(),
        ub: Vec::new(),
        real_rows: Vec::new(),
        real_rhs: Vec::new(),
        c,
        infeasible: true,
        substitutions: Vec::new(),
    }
}

/// The shared pipeline both `simplex.rs` and `interior_point.rs` call
/// directly: Ruiz scaling + redundant-row removal, then up to `rounds`
/// outer repetitions of [`propagate::propagate`] (bound tightening) →
/// [`dualfix`] → row-singleton fixing ([`rowsingleton`]) →
/// doubleton-equality substitution ([`doubleton`], only during the first
/// `doubleton_rounds` outer rounds, once per round) → up to `inner_rounds`
/// *inner* repetitions of [`rowsingleton`] <-> column-singleton
/// substitution ([`colsingleton`]) alone, each stage able to unlock more
/// of the next: a bound propagate tightens can turn an infinite bound
/// finite (letting `dualfix` fix a previously-ineligible variable), and
/// either substitution pass can drop a row or zero out a variable's last
/// remaining appearance (letting `dualfix`'s structural lock-counts, a
/// later outer round, or — the inner loop's own reason to exist — the
/// very next rowsingleton/colsingleton pass in the *same* round find
/// something the one before it never would).
///
/// Both `doubleton`'s own scope and the inner loop's shape were narrowed
/// from an initially broader design, each time based on a full-Netlib
/// measurement, not a priori reasoning: repeating `doubleton` *inside*
/// the inner loop (every pass, alongside rowsingleton/colsingleton) was a
/// net regression (69 of 73 problems slower, +20.8% aggregate) — the
/// interaction actually worth repeating cheaply is specifically
/// rowsingleton<->colsingleton, not doubleton's. Running `doubleton` once
/// per outer round for *every* round (this function's own original
/// design) cost more than it returned on several instances once measured
/// against capping it: restricting it to the first outer round only
/// improved the aggregate (~1.1%) but made a couple of instances
/// (`fffff800`, `tuff`) 2-4x slower; extending that cap to the first two
/// outer rounds recovered those regressions while keeping most of the
/// gain (~9.6% aggregate, 51 of 73 problems faster, only 22 slower) —
/// `doubleton_rounds` is set to `2` at both call sites for this reason.
///
/// `colsingleton` used to be the one piece of this run *before* scaling
/// (in original, unscaled units) as `simplex.rs`'s own separate pre-step;
/// folding it into this scaled pipeline instead means one consistent
/// coordinate system for every reduction here, and lets its own
/// `TOL`-based decisions benefit from scaling's numerical conditioning the
/// same way `dualfix`/`propagate` already do (rather than running on the
/// original problem's raw, possibly very large or very small, coefficient
/// magnitudes).
pub fn run_extended(
    n: usize,
    a: &Csr,
    b: &[f64],
    g: &Csr,
    h: &[f64],
    c: &[f64],
    ruiz_iters: usize,
    prop_passes: usize,
    rounds: usize,
    inner_rounds: usize,
    doubleton_rounds: usize,
) -> ExtendedPresolveResult {
    // One-off, env-var-gated wall-clock breakdown of this function's own
    // major steps — `ENOMOTO_PROF_PHASES`'s `solve_lp_dual` timer starts
    // *after* this whole function returns, so it was blind to presolve's
    // own cost entirely; on several Netlib instances (`ganges`, `stocfor2`,
    // `sierra`) presolve turned out to be 75-92% of *total* solve time,
    // not the simplex loop `ENOMOTO_PROF_PHASES` already covers. A plain
    // local `Instant`/`eprintln!` here (not the atomics-based `timed!`
    // machinery `simplex.rs` uses) is enough since this function runs
    // once per solve, not once per pivot.
    let profile = std::env::var("ENOMOTO_PROF_PRESOLVE").is_ok();
    macro_rules! timed_step {
        ($label:expr, $body:expr) => {{
            if profile {
                let __t0 = std::time::Instant::now();
                let __r = $body;
                eprintln!("  PROF_PRESOLVE {:20} {:8.3}ms", $label, __t0.elapsed().as_secs_f64() * 1e3);
                __r
            } else {
                $body
            }
        }};
    }
    let __wall_t0 = std::time::Instant::now();

    let sc = timed_step!("scaling::compute", scaling::compute(n, a, g, c, ruiz_iters));
    let (mut a, mut g, mut b, mut h, mut c) = timed_step!("scaling::apply", scaling::apply(&sc, a, g, b, h, c));

    let (na, nb) = timed_step!("reduce_equalities", redundancy::reduce_equalities(&a, &b, n));
    a = na;
    b = nb;
    if profile && std::env::var("ENOMOTO_PROF_REDUNDANCY").is_ok() {
        use std::sync::atomic::Ordering::Relaxed;
        let total = redundancy::PROF_TOTAL_STEPS.load(Relaxed);
        let trivial = redundancy::PROF_TRIVIAL_STEPS.load(Relaxed);
        eprintln!(
            "  PROF_REDUNDANCY sparse_steps={total} trivial_steps={trivial} ({:.1}%)",
            100.0 * trivial as f64 / total.max(1) as f64
        );
    }
    let (ng, nh) = timed_step!("reduce_inequalities", redundancy::reduce_inequalities(&g, &h, n));
    g = ng;
    h = nh;

    let mut substitutions: Vec<colsingleton::Substitution> = Vec::new();

    // Fixpoint detection: a round that leaves `a`/`g`'s row counts and
    // every bound unchanged found nothing a further round could act on
    // either (every stage here is a deterministic, pure function of
    // exactly this state), so it's safe to stop before `rounds` even on a
    // round that runs the full stage sequence but accomplishes nothing —
    // this is what turns `rounds` from "run exactly this many times" into
    // "run at most this many times, fewer if convergence comes first".
    let mut prev_signature: Option<(usize, usize, Vec<f64>, Vec<f64>)> = None;
    for round_idx in 0..rounds.max(1) {
        let prop = timed_step!("propagate", propagate::propagate(n, &g, &h, prop_passes));
        if prop.infeasible {
            return extended_infeasible(sc, a, b, c, n);
        }
        let mut lb = prop.lb;
        let mut ub = prop.ub;
        // The inner loop below rebuilds `g` every pass (to fold in
        // `rowsingleton`'s freshest fixes) from *this*, kept up to date
        // after each pass via `extract_bounds` on the just-updated `g` —
        // not left as this round's own initial `propagate` snapshot, which
        // would silently discard every row rewrite doubleton/colsingleton
        // made in an earlier inner pass (see the inner loop's own docs).
        let mut cur_real_rows = prop.real_rows;
        let mut cur_real_rhs = prop.real_rhs;

        let fixes = timed_step!("dualfix", dualfix::fix_dominated_variables(n, &a, &cur_real_rows, &c, &lb, &ub));
        for &(j, value) in &fixes {
            lb[j] = value;
            ub[j] = value;
        }

        // Inner fixpoint: rowsingleton -> colsingleton, up to `inner_rounds`
        // times within this same outer round (before `propagate`/`dualfix`
        // run again) — colsingleton eliminating a variable can turn a row
        // rowsingleton had no reason to touch into a fresh row singleton,
        // and vice versa, the same "later step unlocks an earlier one"
        // logic the outer round loop already relies on, just at a finer
        // grain and without repaying `propagate`'s own cost each time.
        // Stops early on the same row-count fixpoint signature the outer
        // loop uses (cheaper here: `lb`/`ub` aren't touched by doubleton/
        // colsingleton, only by `rowsingleton`'s own `fixes`, already
        // folded in before the signature is taken).
        //
        // `doubleton` itself only runs during the first `doubleton_rounds`
        // *outer* rounds (checked below as `round_idx < doubleton_rounds`),
        // and even then only on the first inner pass — measured on the
        // full Netlib set, both "every outer round" and "every inner pass"
        // were worse than this: repeating it inside this inner loop
        // (alongside rowsingleton/colsingleton every pass) was a net
        // regression (73-problem aggregate +20.8%); running it in every
        // outer round but only once per round (the original design) is
        // this function's own baseline; capping it to the first outer
        // round only improved the aggregate (~1.1%) but made a few
        // instances (`fffff800`, `tuff`) substantially worse; extending
        // that cap to the first *two* outer rounds recovered those
        // regressions while keeping most of the gain (~9.6% aggregate,
        // 51 of 73 problems faster, only 22 slower) — apparently enough
        // rounds for whatever doubleton-eligible structure a typical
        // instance has to be found, without paying for it on every one of
        // up to 10 rounds regardless of whether anything new remains.
        let mut inner_prev_signature: Option<(usize, usize)> = None;
        for _inner in 0..inner_rounds.max(1) {
            let rs = timed_step!("rowsingleton", rowsingleton::fix_singleton_equalities(n, &a, &b, &lb, &ub));
            if rs.infeasible {
                return extended_infeasible(sc, a, b, c, n);
            }
            for &(j, value) in &rs.fixes {
                lb[j] = value;
                ub[j] = value;
            }
            a = rs.a;
            b = rs.b;

            let (ng, nh) = propagate::rebuild_g(n, cur_real_rows, cur_real_rhs, &lb, &ub);
            g = ng;
            h = nh;

            if round_idx < doubleton_rounds.max(1) && _inner == 0 {
                let dbl = timed_step!("doubleton", doubleton::eliminate_doubleton_equalities(n, &a, &b, &g, &h, &c));
                a = dbl.a;
                b = dbl.b;
                g = dbl.g;
                h = dbl.h;
                c = dbl.c;
                // Pin every newly eliminated variable's bounds to `[0, 0]`
                // *now*, not deferred to this function's own end-of-run
                // fix-up (see that fix-up's own docs for the general reason):
                // this inner loop's own next iteration calls `rebuild_g`
                // again with these same `lb`/`ub`, and `rebuild_g` emits a
                // box row for *every* variable with a finite bound with no
                // notion of "already eliminated" — leaving a substituted
                // variable's original bounds live would reintroduce it as a
                // free column for that next `rowsingleton`/`colsingleton`
                // pass to see and (incorrectly) act on again.
                for sub in &dbl.substitutions {
                    lb[sub.var] = 0.0;
                    ub[sub.var] = 0.0;
                }
                substitutions.extend(dbl.substitutions);
            }

            let cs = timed_step!("colsingleton", colsingleton::eliminate_singleton_equalities(n, &a, &b, &g, &h, &c));
            a = cs.a;
            b = cs.b;
            c = cs.c;
            if !cs.substitutions.is_empty() {
                // Drop each eliminated variable's own (now-stale) box-bound
                // rows from `g` before folding in `cs.extra_g_rows` — unlike
                // `doubleton` (which never adds these rows back for an
                // eliminated variable in the first place), `colsingleton`
                // doesn't touch `g`'s existing rows at all, so its eliminated
                // variable's original bound rows would otherwise survive
                // untouched: a "phantom" column with zero cost and no `A`
                // appearances, but *still* carrying its real finite bounds, is
                // free to sit anywhere in that (possibly huge, post-Ruiz-
                // scaling) range without affecting feasibility or the
                // objective — harmless on its own, but exactly the kind of
                // leftover structure that let a *later* round's `dualfix` (see
                // its own module docs on this) or a subsequent chain step
                // reason about this variable as if it still had independent
                // degrees of freedom, instead of the single value its own
                // substitution now fully determines.
                let eliminated_this_pass: std::collections::BTreeSet<usize> = cs.substitutions.iter().map(|s| s.var).collect();
                // Same reason as `dbl.substitutions` above: pin now, not
                // deferred, so this inner loop's next `rebuild_g` call
                // sees these variables as fixed rather than reintroducing
                // their original bounds as a live box row.
                for &j in &eliminated_this_pass {
                    lb[j] = 0.0;
                    ub[j] = 0.0;
                }
                let gr = g.as_ref();
                let mut g_rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(gr.nrows() + cs.extra_g_rows.len());
                let mut h_vec: Vec<f64> = Vec::with_capacity(gr.nrows() + cs.extra_h.len());
                for i in 0..gr.nrows() {
                    let row: Vec<(usize, f64)> = gr.col_indices_of_row(i).zip(gr.values_of_row(i)).map(|(j, &v)| (j, v)).collect();
                    if row.len() == 1 && eliminated_this_pass.contains(&row[0].0) {
                        continue;
                    }
                    g_rows.push(row);
                    h_vec.push(h[i]);
                }
                g_rows.extend(cs.extra_g_rows);
                h_vec.extend(cs.extra_h);
                g = csr_from_rows(&g_rows, n);
                h = h_vec;
            }
            substitutions.extend(cs.substitutions);

            let inner_signature = (a.nrows(), g.nrows());
            if inner_prev_signature == Some(inner_signature) {
                break;
            }
            inner_prev_signature = Some(inner_signature);

            // Re-split the just-updated `g` back into its real (multi-
            // variable) rows for the *next* inner pass's own `rebuild_g`
            // call — cheap (a single scan of `g`, not a re-run of
            // `propagate`'s own activity-bound derivation), and necessary:
            // without it the next pass would silently discard every row
            // rewrite `doubleton`/`colsingleton` just made, reintroducing
            // this pass's already-eliminated variables into a stale copy
            // of the original row. A row that substitution collapsed from
            // multi-variable down to a single variable (e.g. `-a-2b<=-6`
            // becoming `-3a<=-6` once `b=a` is substituted in) comes back
            // from `extract_bounds` as a *bound*, not a row — folded into
            // `lb`/`ub` here (tighter of the two) rather than discarded,
            // since dropping it would silently lose a real constraint.
            let (refreshed_lb, refreshed_ub, refreshed_real_rows, refreshed_real_rhs) = propagate::extract_bounds(n, &g, &h);
            for j in 0..n {
                lb[j] = lb[j].max(refreshed_lb[j]);
                ub[j] = ub[j].min(refreshed_ub[j]);
            }
            cur_real_rows = refreshed_real_rows;
            cur_real_rhs = refreshed_real_rhs;
        }

        let signature = (a.nrows(), g.nrows(), lb.clone(), ub.clone());
        if prev_signature.as_ref() == Some(&signature) {
            break;
        }
        prev_signature = Some(signature);
    }

    let prop = timed_step!("final propagate", propagate::propagate(n, &g, &h, prop_passes));
    if profile {
        eprintln!("PROF_PRESOLVE total {:.3}ms", __wall_t0.elapsed().as_secs_f64() * 1e3);
    }
    if prop.infeasible {
        return ExtendedPresolveResult {
            scaling: sc,
            a,
            b,
            g: prop.g,
            h: prop.h,
            lb: prop.lb,
            ub: prop.ub,
            real_rows: prop.real_rows,
            real_rhs: prop.real_rhs,
            c,
            infeasible: true,
            substitutions,
        };
    }

    // Every eliminated variable's true value is recovered later purely via
    // `Substitution::value` (see this struct's own docs) — pinned to a
    // single arbitrary finite point *here*, before `g`/`h` are derived,
    // rather than left to each caller to notice and fix up on its own
    // (`simplex.rs` used to do this itself, straight on `pre.lb`/`pre.ub`,
    // after calling this function — still does, harmlessly redundantly,
    // now that it's already done here). Without this, a caller that reads
    // `g`/`h` directly (rather than `lb`/`ub`/`real_rows`/`real_rhs`, the
    // split form `simplex.rs` prefers) would see an eliminated variable as
    // a genuinely free, zero-cost, zero-appearance column — which is
    // exactly the "phantom column with independent degrees of freedom"
    // shape `colsingleton`'s own module docs warn a *later* presolve round
    // could be confused by, and which would make an interior-point
    // method's KKT system singular in that column outright.
    let mut lb = prop.lb;
    let mut ub = prop.ub;
    for sub in &substitutions {
        lb[sub.var] = 0.0;
        ub[sub.var] = 0.0;
    }
    let (g, h) = propagate::rebuild_g(n, prop.real_rows.clone(), prop.real_rhs.clone(), &lb, &ub);

    ExtendedPresolveResult {
        scaling: sc,
        a,
        b,
        g,
        h,
        lb,
        ub,
        real_rows: prop.real_rows,
        real_rhs: prop.real_rhs,
        c,
        infeasible: false,
        substitutions,
    }
}
