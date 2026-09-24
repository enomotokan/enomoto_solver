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
//!      and duplicate/dominated rows from the scaled `(G, h)`. Both run once
//!      here, before the round loop below — but [`redundancy::reduce_inequalities`]
//!      (a cheap O(nnz) hash pass, unlike [`redundancy::reduce_equalities`]'s
//!      expensive QR/Gaussian elimination) also runs again at the *end* of
//!      every outer round, since `doubleton`/`colsingleton` rewrite `g`'s real
//!      inequality rows during substitution and can turn two originally-
//!      distinct rows into duplicates this pre-loop call could never have
//!      seen (see that call site's own docs).
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
//!      see. Every stage here retries on *every* outer pass — a genuine
//!      fixpoint loop over the whole reduction set (the same "keep
//!      retrying each individual reduction until none of them find
//!      anything" shape HiGHS's own presolve driver uses), not a loop with
//!      one technique singled out for an early, hardcoded cutoff — except
//!      `doubleton`, which is retried every pass only *until* one of its
//!      own calls finds nothing, then latches off for the rest of this
//!      run (see [`run_extended`]'s own docs, and `doubleton_active`'s,
//!      for why a fixed round cap and an unconditional per-round retry
//!      were each measured worse than this latch on the full Netlib set).
//!      All of these stop early, before their own cap, once a pass changes
//!      neither row count nor (for the outer loop) any bound — a fixpoint:
//!      every stage here is a deterministic function of exactly that
//!      state, so a pass that changes nothing leaves nothing for a further
//!      pass to find either.
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

pub mod aggregator;
pub mod colsingleton;
pub mod doubleton;
pub mod dominatedcol;
pub mod dualfix;
pub mod dualpropagate;
pub mod foldfixed;
pub mod freevar;
pub mod ineqsingleton;
pub mod parallelcols;
pub mod parallelrows;
pub mod propagate;
pub mod redundancy;
pub mod rowdominance;
pub mod rowsingleton;
pub mod scaling;
pub mod smallcoeff;
pub mod sparsify;
pub mod stuffing;

use crate::sparse::{Csr, csr_from_rows, csr_row_vec, csr_rows};
use crate::types::{ConstraintRow, RowSense, VariableData};
use scaling::Scaling;

/// Builds `A x = b`, `G x <= h` (bounds folded into `G` as single-variable
/// rows — `ub` as `(j, 1.0)`/`h=ub`, `lb` as `(j, -1.0)`/`h=-lb` — emitted
/// only for a *finite* bound: an infinite one is a genuine absence of a
/// constraint, not a very large `h`, and `propagate::extract_bounds`
/// already treats a variable with no such row as unbounded on that side
/// by default, so there is nothing for an `h = +/-inf` row to add other
/// than a value every downstream arithmetic pass (`redundancy`/`scaling`)
/// would otherwise have to special-case) directly from the model's
/// variables/constraints. Shared by `interior_point::qp::build` (which
/// adds its own `c` from `obj_coeffs_for_min`) and `simplex.rs`'s presolve
/// entry point.
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
        if v.ub.is_finite() {
            g_rows.push(vec![(j, 1.0)]);
            h.push(v.ub);
        }
        if v.lb.is_finite() {
            g_rows.push(vec![(j, -1.0)]);
            h.push(-v.lb);
        }
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
    /// `true` iff [`freevar::eliminate_free_variables`] found a free
    /// variable that is genuinely unbounded — either no remaining
    /// appearance anywhere (`A`'s rows or the real inequality rows) with a
    /// nonzero objective coefficient, or exactly one inequality-row
    /// appearance whose sign combination with that coefficient leaves it
    /// unbounded on the objective-favored side (see that function's own
    /// docs for both) — and `a`/`b`/`c`/`lb`/`ub`/etc. below must not be
    /// trusted. Mutually exclusive with `infeasible` — presolve reports at
    /// most one of the two.
    pub unbounded: bool,
    /// Every eliminating step from every technique in this pipeline
    /// (`doubleton`/`colsingleton`/`aggregator`/`freevar`'s own ordinary
    /// [`colsingleton::Substitution`]s, and every
    /// [`parallelcols::merge_parallel_columns`] fold), in one single
    /// chronological log — a caller recovers every original variable's true
    /// value by walking this log in **reverse** (see each variant's own
    /// `value`/`apply`) — the exact reverse of the order these steps ran in
    /// presolve.
    ///
    /// **Must stay one interleaved log, not two separate per-kind lists
    /// undone in two separate passes** (an earlier version of this struct
    /// had exactly that: `substitutions: Vec<colsingleton::Substitution>`
    /// plus a fully separate `parallel_col_substitutions`, undone as two
    /// back-to-back loops). That earlier design's own reasoning — "a column
    /// `parallelcols` eliminates has zero remaining row appearances the
    /// instant it's eliminated, so no *later* round's row-based
    /// substitution can ever reference it as a term" — is true but answers
    /// the wrong direction: it rules out a later ordinary substitution
    /// referencing an already-*eliminated* column, not an *earlier*
    /// substitution referencing a column `parallelcols` merges away
    /// *afterward*. `parallelcols` only ever removes one of a merged pair
    /// (`eliminated`) — the other (`kept`) survives at the *same* index,
    /// now holding a composite value, and any ordinary substitution
    /// recorded *before* that merge whose `terms` reference `kept` is still
    /// sitting in the (undone-first, in the old two-pass design) ordinary
    /// list, so it read `kept`'s post-merge composite value instead of the
    /// real pre-merge one. Confirmed as the exact mechanism behind a false
    /// wrong-objective result on Netlib `greenbea` (see
    /// `parallelcols-postsolve-order-bug` project memory): resolving one
    /// chronological log in one reverse pass (`PostsolveStep` below) is the
    /// fix — see [`PostsolveStep`]'s own docs.
    pub postsolve_log: Vec<PostsolveStep>,
}

/// One step of [`ExtendedPresolveResult::postsolve_log`] — either kind of
/// elimination this pipeline performs, kept in one shared chronological
/// order specifically so postsolve can undo them in a single reverse pass
/// (see that field's own docs for why two separate per-kind passes is
/// unsound). The two variants' own recovery shapes stay genuinely
/// different — `Sub`'s [`colsingleton::Substitution::value`] is a one-way
/// linear formula from already-known inputs, `ParallelCol`'s
/// [`parallelcols::Substitution::apply`] both reads and rewrites `kept`'s
/// own slot — this enum only unifies the *order* they're resolved in, not
/// how each one resolves.
pub enum PostsolveStep {
    Sub(colsingleton::Substitution),
    ParallelCol(parallelcols::Substitution),
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
        unbounded: false,
        postsolve_log: Vec::new(),
    }
}

fn extended_unbounded(sc: Scaling, a: Csr, b: Vec<f64>, c: Vec<f64>, n: usize) -> ExtendedPresolveResult {
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
        infeasible: false,
        unbounded: true,
        postsolve_log: Vec::new(),
    }
}

/// The shared pipeline both `simplex.rs` and `interior_point.rs` call
/// directly: Ruiz scaling + redundant-row removal, then up to `rounds`
/// outer repetitions of [`propagate::propagate`] (bound tightening) →
/// [`dualfix`] → row-singleton fixing ([`rowsingleton`]) →
/// doubleton-equality substitution ([`doubleton`], once per outer round,
/// until it latches off — see `doubleton_active`) → up to `inner_rounds`
/// *inner* repetitions of [`rowsingleton`] <-> column-singleton
/// substitution ([`colsingleton`]) alone, each stage able to unlock more
/// of the next: a bound propagate tightens can turn an infinite bound
/// finite (letting `dualfix` fix a previously-ineligible variable), and
/// either substitution pass can drop a row or zero out a variable's last
/// remaining appearance (letting `dualfix`'s structural lock-counts, a
/// later outer round, or — the inner loop's own reason to exist — the
/// very next rowsingleton/colsingleton pass in the *same* round find
/// something the one before it never would). The outer loop is a genuine
/// fixpoint (see `prev_signature` below): it keeps re-running this whole
/// stage sequence until one full round changes neither a row/column count
/// nor any bound — the "retry every individual reduction until none of
/// them find anything left" loop shape HiGHS's own `HPresolve::run` uses
/// (`docs/` — see the HiGHS presolve summary) — except for `doubleton`
/// itself, which additionally *latches off* the moment one of its own
/// calls finds nothing (rather than being retried every remaining round
/// regardless, the way `rowsingleton`/`colsingleton` are): see
/// `doubleton_active`'s own docs for why.
///
/// This *reworks* an earlier, narrower version of this function that
/// capped `doubleton` to a fixed first two outer rounds
/// (`doubleton_rounds = 2`) after measuring that running it every round —
/// this function's own original design — cost more than it returned on
/// several Netlib instances relative to that fixed cap (~9.6% aggregate
/// faster capped, 51 of 73 problems, at the time of that measurement). A
/// full re-measurement of the *uncapped* (every round, unconditionally)
/// version reproduced that regression on this codebase (+6.9% aggregate
/// wall-clock over 73 Netlib problems, 38 slower / 32 faster, iteration
/// counts essentially unchanged at -0.1% — see this crate's own benchmark
/// CSVs), with the worst regressions landing on problems that presolve
/// converges on quickly (`recipe` +332%, `israel` +315%, `cycle` +133%):
/// exactly the case where a fixed number of *always*-paid full-matrix
/// scans costs more than the reduction opportunities left to find. The
/// latch here is the fix for that: it keeps `doubleton`'s own scan a true
/// per-technique fixpoint (never capped at a fixed round count picked in
/// advance, unlike the reverted design) while still stopping the moment
/// it stops paying for itself (unlike the plain uncapped version), rather
/// than either. Repeating `doubleton` *inside* the inner
/// rowsingleton<->colsingleton loop (every inner pass, not just once per
/// outer round) remains a separate, still-net-negative idea (69 of 73
/// problems slower, +20.8% aggregate when tried) and is not what this
/// does — `doubleton` still runs at most once per outer round, on that
/// round's first inner pass.
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

    // Raw bounds straight off the just-scaled `g`/`h` (no propagation yet —
    // that only starts inside the round loop below), handed to
    // `reduce_equalities` purely for its own block-decomposition pre-pass's
    // small-coefficient edge filter (see that pre-pass's own docs on why an
    // unpropagated, looser bound here is safe, just more conservative, than
    // the fully-tightened `orig_lb`/`orig_ub` extracted again below).
    let (pre_lb, pre_ub) = propagate::extract_bounds_only(n, &g, &h);
    let (na, nb) = timed_step!("reduce_equalities", redundancy::reduce_equalities(&a, &b, n, &pre_lb, &pre_ub));
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

    // `parallelrows::merge_parallel_rows` and `rowdominance::find_dominated_rows`
    // (Andersen & Andersen 1995) were implemented, unit-tested, and wired in
    // right here for a full-Netlib A/B measurement — then removed again,
    // mirroring `dominatedcol`/`sparsify`'s own precedent (see either
    // module's own docs): both fired on **zero** of the 73 in-scope Netlib
    // instances (instrumented directly, not inferred from timing alone),
    // so wiring them in was pure candidate-search tax for no reduction
    // anywhere — aggregate `ours` time 3.85s unwired vs. 3.91s wired
    // (+1.5%), 73/73 objective values unchanged either way. Left in the
    // module tree, tested, for the reason each docstring gives (future
    // problem shapes / a relaxed finite-bounds invariant), not deleted.
    //
    // Re-measured 2026-09-20 against the current pipeline (post single
    // dual-path unification, freevar elimination, DSE refresh-on-refactor —
    // all postdate the original measurement above): all six previously-
    // shelved reductions (`parallelrows`, `rowdominance`, `dominatedcol`,
    // `stuffing` here plus `sparsify` once per outer round right before
    // `colsingleton` and `smallcoeff` once at the very end) wired in
    // together, full 73-problem set, 73/73 still solved to optimality with
    // matching objectives either way (the `smallcoeff`-wiring `perold`
    // crash this module's docs warned of did *not* reproduce under the
    // current pipeline) — but aggregate `ours` time still regressed, 4.248s
    // unwired vs. 5.002s wired (+17.8%), concentrated on the same kind of
    // degenerate/shape-sensitive instances this crate's history keeps
    // finding for structural presolve changes: `cycle` +289%, `perold`
    // +88%, `wood1p` +55%. 10 of 73 instances improved (best: `scfxm2`
    // -23%), nowhere near enough to offset the losses. Conclusion
    // unchanged: left unwired.

    // Frozen once, before any round's `propagate` call ever runs — see
    // `dualpropagate::run`'s own docs for why it needs the model's
    // *original* bounds specifically, not whatever `lb`/`ub` a later
    // round's own activity-based tightening has since narrowed them to.
    let (orig_lb, orig_ub) = propagate::extract_bounds_only(n, &g, &h);

    let mut postsolve_log: Vec<PostsolveStep> = Vec::new();

    // Latches off permanently the first time a round's `doubleton` call
    // finds nothing: unlike `rowsingleton`/`colsingleton` (cheap enough to
    // keep re-trying every round regardless), `doubleton`'s own full-matrix
    // scan is exactly the cost the measurement in this function's own docs
    // found *not* worth paying once it stops finding anything — this stops
    // calling it the moment that happens, rather than either an arbitrary
    // fixed round cap (the earlier, narrower `doubleton_rounds` design) or
    // paying for the scan on every one of up to `rounds` rounds regardless
    // of whether a later round's `propagate`/`dualfix`/`rowsingleton`/
    // `colsingleton` reductions could in principle re-expose a new
    // doubleton-equality row (measured to be rare enough in practice that
    // this one-way latch is worth its own presolve-time savings).
    let mut doubleton_active = true;
    // Consecutive empty calls before the latch engages (`ENOMOTO_T_DOUBLETON_STRIKES`).
    let mut doubleton_empty_streak = 0usize;

    // Same one-way latch, same reason: `dualpropagate`'s own transpose-and-
    // propagate call is a full-matrix pass, worth skipping once a round's
    // call finds neither a new implied-equality row to promote nor a new
    // column to fix (its two reductions — see `dualpropagate::run`'s docs).
    let mut dualpropagate_active = true;
    let mut dualpropagate_empty_streak = 0usize;

    // Same one-way latch, same reason again: `parallelcols`'s own
    // signature-grouping scan is a full-matrix pass (see its own module
    // docs' "Candidate search" section), worth skipping once it stops
    // finding anything. *Not* a simple one-strike latch like
    // `doubleton_active` above, though — a 2026-09-20 survey of every one
    // of the 73 in-scope Netlib instances found `ganges`'s own first
    // nonzero-elimination round is round *2*, its round 1 finding nothing
    // at all (only `czprob`/`greenbea` find something on round 1 itself,
    // then again later): a one-strike version of this latch turned off
    // after `ganges`'s own empty round 1, before round 2 — the one that
    // actually matters for it — ever ran, silently losing that instance's
    // entire reduction (caught by re-running this exact survey after
    // first writing this latch as one-strike). Requires *two consecutive*
    // empty rounds before disengaging instead — `ganges` alone would still
    // cost one avoidable extra call across its own 9 rounds, judged not
    // worth a third latch state to also chase down.
    let mut parallelcols_active = true;
    let mut parallelcols_empty_streak = 0usize;

    // Fixpoint detection: a round that leaves `a`/`g`'s row counts and
    // every bound unchanged found nothing a further round could act on
    // either (every stage here is a deterministic, pure function of
    // exactly this state), so it's safe to stop before `rounds` even on a
    // round that runs the full stage sequence but accomplishes nothing —
    // this is what turns `rounds` from "run exactly this many times" into
    // "run at most this many times, fewer if convergence comes first".
    let mut prev_signature: Option<(usize, usize, Vec<f64>, Vec<f64>)> = None;
    let mut prev_struct: Option<(usize, usize, usize, usize)> = None;
    let mut eqprop_idle = false;
    for _round_idx in 0..rounds.max(1) {
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

        // Equality-row counterpart of the `propagate` call above (see
        // `propagate::propagate_equalities`'s own docs). Run for the first
        // two outer rounds — the round count that already captured every
        // forcing row and bound tightening in the 93-problem measurement
        // (analysis/greenbea_20260921_230908.md §7.1); later rounds found
        // nothing further but still paid for the full-matrix scan.
        //
        // Tried running this pass *after* `dualpropagate`/`aggregator`/
        // `parallelcols` instead, so it would only bound whatever column
        // none of those three could already eliminate outright (they each
        // need a column still unbounded on their own side — see every
        // module's own docs). That ordering did preserve more of
        // `aggregator`'s reach on `stocfor2` (its own motivating instance),
        // but cost most of `greenbea`'s win (M-tracked columns 3,569 -> ~2k
        // instead of ~300, since several outer rounds' worth of `aggregator`
        // substitutions consume equality rows this pass would otherwise
        // have used) and, measured on the full 93-problem set, a *worse*
        // aggregate `ours` time than running here (52.1s vs 50.9s) despite
        // "fixing" `stocfor2` partway — `pilot87`/`d2q06c` regressed instead
        // under that ordering. Running here, before `dualfix`, remains the
        // best aggregate result found; `stocfor2`'s own regression (~+90%,
        // absolute ~0.13s) is this trade-off's known remaining cost — see
        // this function's own module docs and the loop's analysis file for
        // the full comparison table.
        //
        // `ENOMOTO_T_EQPROP_SKIP_IDLE=1` (default 0): once a round's call
        // reports no forcing row, fixed column or tightened bound, skip the
        // remaining eqprop rounds (C20; not guaranteed identical — a later
        // round starts from tighter bounds and could still find something).
        if _round_idx < tunable!("ENOMOTO_T_EQPROP_ROUNDS", 2, usize) && !eqprop_idle {
            let eq = timed_step!("eqprop", propagate::propagate_equalities(&a, &b, &mut lb, &mut ub, prop_passes));
            if eq.infeasible {
                return extended_infeasible(sc, a, b, c, n);
            }
            if tunable!("ENOMOTO_T_EQPROP_SKIP_IDLE", 0, usize) != 0 && eq.forcing_rows == 0 && eq.fixed_cols == 0 && eq.tightened == 0 {
                eqprop_idle = true;
            }
        }

        let fixes = timed_step!("dualfix", dualfix::fix_dominated_variables(n, &a, &cur_real_rows, &c, &lb, &ub));
        for &(j, value) in &fixes {
            lb[j] = value;
            ub[j] = value;
        }

        // `stuffing::fix_singleton_columns` was implemented, unit-tested,
        // and wired in right here for a full-Netlib A/B measurement — then
        // removed again, joining `dominatedcol`/`sparsify`/`parallelrows`/
        // `rowdominance` (see the block above and each module's own docs):
        // it fired on **zero** of the 73 in-scope Netlib instances
        // (instrumented directly), and its own timing was within this
        // machine's measured ~20% run-to-run noise band either way (three
        // repeats each, total `ours` time across 72 of the 73 problems:
        // 3.40-4.25s unwired vs. 3.68-3.97s wired — `cycle` excluded from
        // this comparison since it reports a wrong objective in *both*
        // configurations, a pre-existing bug unrelated to this pass; see
        // this crate's own benchmark notes). Exactly the shape the paper
        // itself predicts (Gamrath et al. 2015, §6): on a generic MIPLIB-
        // style test set stuffing fires on well under a quarter of
        // instances and fixes under 1% of variables even then — the
        // technique is aimed at supply-chain-shaped models with many
        // flexible-slack singleton columns, a structure no Netlib LP here
        // happens to have. Left in the module tree, tested, for the same
        // reason as its neighbors: a future problem shape (or this crate
        // someday handling true MIP columns, where the paper's own
        // reported gains were largest) could still exercise it.

        // Two reductions off one dual-feasibility propagation (see
        // `dualpropagate`'s own docs for both): promote every inequality
        // row it proves tight in every optimal solution into the equality
        // system outright — `doubleton`/`colsingleton`/`rowsingleton`
        // already know what to do with a true equality, so this needs no
        // new substitution logic of its own, just a relabeling of which
        // system a row lives in before those passes run below — and fix
        // every column whose reduced cost the same propagated dual box
        // proves one-signed everywhere (HiGHS's own "dominated column";
        // see `dualpropagate`'s own "Column fixing" docs section), applied
        // the same way `dualfix`'s own fixes are just above.
        if dualpropagate_active {
            let dual_red = timed_step!("dualpropagate", dualpropagate::run(n, &a, &cur_real_rows, &c, &lb, &ub, &orig_lb, &orig_ub, prop_passes));
            if dual_red.implied_equalities.is_empty() && dual_red.fixed_columns.is_empty() {
                dualpropagate_empty_streak += 1;
                if dualpropagate_empty_streak >= tunable!("ENOMOTO_T_DUALPROPAGATE_STRIKES", 1, usize) {
                    dualpropagate_active = false;
                }
            } else {
                dualpropagate_empty_streak = 0;
                if std::env::var("ENOMOTO_DEBUG_DUALPROPAGATE").is_ok() {
                    eprintln!("DEBUG_DUALPROPAGATE: implied_equalities={} fixed_columns={}", dual_red.implied_equalities.len(), dual_red.fixed_columns.len());
                }
                for &(j, value) in &dual_red.fixed_columns {
                    lb[j] = value;
                    ub[j] = value;
                }
                if !dual_red.implied_equalities.is_empty() {
                    let implied = dual_red.implied_equalities;
                    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(&a);
                    let mut new_b = b.clone();
                    let mut promoted = vec![false; cur_real_rows.len()];
                    for &gi in &implied {
                        promoted[gi] = true;
                        a_rows.push(cur_real_rows[gi].clone());
                        new_b.push(cur_real_rhs[gi]);
                    }
                    a = csr_from_rows(&a_rows, n);
                    b = new_b;
                    let mut kept_rows = Vec::with_capacity(cur_real_rows.len() - implied.len());
                    let mut kept_rhs = Vec::with_capacity(cur_real_rhs.len() - implied.len());
                    for (i, (row, rhs)) in cur_real_rows.into_iter().zip(cur_real_rhs.into_iter()).enumerate() {
                        if !promoted[i] {
                            kept_rows.push(row);
                            kept_rhs.push(rhs);
                        }
                    }
                    cur_real_rows = kept_rows;
                    cur_real_rhs = kept_rhs;
                }
            }
        }

        // Column singletons in inequality/ranged rows (see `ineqsingleton`'s
        // docs): fix the column at a bound, or turn its row into an
        // equality that `colsingleton` below then substitutes out.
        if std::env::var("ENOMOTO_INEQ_SINGLETON").is_ok() {
            let isr = timed_step!("ineqsingleton", ineqsingleton::run(n, &a, &cur_real_rows, &cur_real_rhs, &c, &lb, &ub));
            if std::env::var("ENOMOTO_DEBUG_INEQ_SINGLETON").is_ok() {
                eprintln!("DEBUG_INEQ_SINGLETON: fixes={} implied_equalities={}", isr.fixes.len(), isr.implied_equalities.len());
            }
            for &(j, value) in &isr.fixes {
                lb[j] = value;
                ub[j] = value;
            }
            if !isr.implied_equalities.is_empty() {
                let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(&a);
                let mut drop = vec![false; cur_real_rows.len()];
                for &(gi, other) in &isr.implied_equalities {
                    a_rows.push(cur_real_rows[gi].clone());
                    b.push(cur_real_rhs[gi]);
                    drop[gi] = true;
                    if let Some(o) = other {
                        drop[o] = true;
                    }
                }
                a = csr_from_rows(&a_rows, n);
                let mut kept_rows = Vec::with_capacity(cur_real_rows.len());
                let mut kept_rhs = Vec::with_capacity(cur_real_rhs.len());
                for (i, (row, rhs)) in cur_real_rows.into_iter().zip(cur_real_rhs.into_iter()).enumerate() {
                    if !drop[i] {
                        kept_rows.push(row);
                        kept_rhs.push(rhs);
                    }
                }
                cur_real_rows = kept_rows;
                cur_real_rhs = kept_rhs;
            }
        }

        // Fold every column fixed so far (by `dualfix`/`dualpropagate` just
        // above, by an earlier round's `rowsingleton`, or from the model's
        // own input bounds) straight out of `A`'s equality rows and `G`'s
        // real inequality rows — see `foldfixed`'s own docs for why this
        // needs its own pass: fixing a bound alone leaves a row's *literal*
        // term count unchanged, which would otherwise hide a row that just
        // became a genuine `rowsingleton`/`doubleton` candidate (or short
        // enough for `aggregator`'s implied-free gate) behind stale dead
        // weight until some *later* round's `extract_bounds` call happened
        // to notice. Run unconditionally every round rather than gated on
        // "did anything get fixed this round" — same reasoning as
        // `reduce_inequalities(round)`'s own unconditional placement
        // further down: an O(nnz) scan cheap enough that the bookkeeping
        // to skip it (correctly, across every source of a fix — including
        // `rowsingleton`'s own, decided later in this same round's inner
        // loop and easy to under-count here) isn't worth it.
        let a_rows: Vec<Vec<(usize, f64)>> = csr_rows(&a);
        let fold_a = timed_step!("foldfixed(A)", foldfixed::fold_fixed_columns(&a_rows, &b, &lb, &ub, RowSense::Eq));
        if fold_a.infeasible {
            return extended_infeasible(sc, a, b, c, n);
        }
        a = csr_from_rows(&fold_a.rows, n);
        b = fold_a.rhs;

        let fold_g = timed_step!("foldfixed(G)", foldfixed::fold_fixed_columns(&cur_real_rows, &cur_real_rhs, &lb, &ub, RowSense::Le));
        if fold_g.infeasible {
            return extended_infeasible(sc, a, b, c, n);
        }
        cur_real_rows = fold_g.rows;
        cur_real_rhs = fold_g.rhs;

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
        // `doubleton` runs on every outer round's first inner pass until it
        // latches off (`doubleton_active`, see its own docs above) — no
        // longer gated by a fixed round count (see this function's own
        // docs for the earlier, narrower `doubleton_rounds`-capped version
        // this reworks). Repeating it on every one of this inner loop's
        // own passes (alongside rowsingleton/colsingleton) remains the
        // separately-measured net regression described there and is still
        // not done — only the *outer*-round gate was replaced with the
        // latch.
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

            let (ng, nh) = timed_step!("rebuild_g(inner)", propagate::rebuild_g_ref(n, &cur_real_rows, &cur_real_rhs, &lb, &ub));
            g = ng;
            h = nh;
            // Whether `(g, h)` is still exactly `rebuild_g_ref(cur_real_rows,
            // cur_real_rhs, lb, ub)` — see the `extract_bounds(inner)` skip
            // below.
            let mut g_is_rebuilt = true;

            if _inner == 0 && doubleton_active {
                let dbl = timed_step!("doubleton", doubleton::eliminate_doubleton_equalities(n, &a, &b, &g, &h, &c));
                if dbl.substitutions.is_empty() {
                    doubleton_empty_streak += 1;
                    if doubleton_empty_streak >= tunable!("ENOMOTO_T_DOUBLETON_STRIKES", 1, usize) {
                        doubleton_active = false;
                    }
                } else {
                    doubleton_empty_streak = 0;
                }
                if !dbl.unchanged {
                    g_is_rebuilt = false;
                }
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
                postsolve_log.extend(dbl.substitutions.into_iter().map(PostsolveStep::Sub));
            }

            let cs = timed_step!("colsingleton", colsingleton::eliminate_singleton_equalities(n, &a, &b, &g, &h, &c));
            let cs_unchanged = cs.substitutions.is_empty();
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
                    let row: Vec<(usize, f64)> = csr_row_vec(&g, i);
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
            postsolve_log.extend(cs.substitutions.into_iter().map(PostsolveStep::Sub));

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
            //
            // Skipped when it would be an exact round trip: `g` is still the
            // `rebuild_g_ref` output of this pass (no doubleton rewrite, no
            // colsingleton substitution since), and every real row is
            // already in the form `rebuild_g_ref` stores it (not a single
            // entry, strictly ascending columns, no stored zero) — then
            // `extract_bounds` would hand back `lb`/`ub` and the real rows
            // and rhs bit for bit.
            let round_trip = g_is_rebuilt && cs_unchanged && cur_real_rows.iter().all(|r| r.len() != 1 && r.iter().all(|&(_, v)| v != 0.0) && r.windows(2).all(|w| w[0].0 < w[1].0));
            if !round_trip {
                let (refreshed_lb, refreshed_ub, refreshed_real_rows, refreshed_real_rhs) = timed_step!("extract_bounds(inner)", propagate::extract_bounds(n, &g, &h));
                for j in 0..n {
                    lb[j] = lb[j].max(refreshed_lb[j]);
                    ub[j] = ub[j].min(refreshed_ub[j]);
                }
                cur_real_rows = refreshed_real_rows;
                cur_real_rhs = refreshed_real_rhs;
            }
        }

        // Aggregator (HiGHS's own name; `HPresolve::aggregator`): eliminates
        // every column a single one of `A`'s own equality rows already
        // proves implied-free — its box bound already forced redundant by
        // that row's own activity — reachable through >= 2 such rows, with
        // no `colsingleton`-style bound-preservation row ever needed (see
        // `aggregator`'s own module docs for the full history: a first
        // version with no implied-free gate at all was a severe regression;
        // a second, cross-row-aggregate version fixed that but produced a
        // false `Unbounded` on `shell`, caught by this crate's own
        // objective-mismatch check; this row-local version is the one
        // that's actually correct and wired in). Placed right here,
        // mirroring HiGHS's own placement in `HPresolve::presolve`
        // (`HPresolve.cpp:5901-5917`: right after its fast singleton/
        // doubleton loop converges, each outer main-loop iteration) — this
        // crate's own outer round loop already re-enters
        // `rowsingleton`/`doubleton`/`colsingleton`'s inner fixpoint on the
        // next round whenever this call changes anything (via the
        // `signature` fixpoint check below), giving the same "called
        // repeatedly, cascading with the fast loop" behavior HiGHS's own
        // `problemSizeReduction() > 0.05 -> continue` re-entry achieves,
        // without needing a separate re-entry trigger of its own.
        //
        // Measured on the full 73-problem Netlib set (`ENOMOTO_DISABLE_AGGREGATOR`
        // A/B, two runs each way): aggregate `ours` time is statistically
        // indistinguishable from unwired (4.326-4.335s wired vs.
        // 4.215-4.343s unwired — well inside this machine's own ~3% run-to-
        // run spread), 73/73 objectives matching either way (`cycle`'s own
        // pre-existing ~3e-4 mismatch unaffected). Individual instances
        // move more than that noise band in both directions: `stocfor2`
        // (this module's own motivating instance) -34% (0.191s -> 0.126s,
        // iterations 1665 -> ~1550), `shell` -30%, `sc205`/`scrs8`/
        // `scorpion` -21% to -29%; `maros` +59% (951 -> 889 iterations, but
        // each one costlier — the same fill-in-vs-iteration-count tradeoff
        // this crate's history keeps finding on specific instances),
        // `recipe`/`finnis`/`capri`/`agg3`/`scagr25`/`scfxm3` +18-49% (all
        // small in absolute time). Net: worth keeping wired in, unlike this
        // module's own two earlier reverted attempts — genuine, reproducible
        // wins on several instances against a wash everywhere else, not a
        // one-sided regression.
        // `aggregator`'s cross-row generalization
        // (`eliminate_implied_free_columns_xrow`, see that function's own
        // docs) fixes the correctness bug that sank an *earlier* cross-row
        // attempt (a false `Unbounded` on `shell`, root-caused: a
        // candidate's justification is now recomputed from live rows with
        // current content immediately before elimination, never trusted
        // from the snapshot candidate-generation pass — see that
        // function's own docs for the exact mechanism and the numeric
        // trace on `shell`), and is available here via `ENOMOTO_XROW_AGGREGATOR`.
        //
        // **Stays opt-in, row-local stays default** — a first measurement
        // (2026-09-22) was accidentally taken on a stale feature branch 44
        // commits behind `main` (missing this file's own `eqprop`
        // wiring above and other propagate/dualpropagate strengthening),
        // where `greenbea` alone was pathologically slow (3,569 structural
        // columns left genuinely unbounded post-presolve, ~9-11s) and
        // dominated the 93-problem aggregate enough to show cross-row as a
        // reproducible ~12% win. Re-measured on actual `main` (3 reps each,
        // `greenbea` correctly down to 367 unbounded columns / ~0.6s there):
        // row-local 54.69s/53.49s/52.02s (mean ~53.4s) vs. cross-row
        // 56.65s/50.25s/55.99s (mean ~54.3s) — the two configs' ranges
        // overlap and the means are within a percent of each other, i.e. a
        // wash, not the clean win the stale-branch measurement showed. 0/93
        // objective mismatches in every rep either way. Kept opt-in rather
        // than flipping the default a second time on inconclusive numbers —
        // see `presolve-fxhash-and-rebuildg-measured` and the follow-up
        // memory on this specific mismeasurement for the full writeup.
        let agg = if std::env::var("ENOMOTO_DISABLE_AGGREGATOR").is_ok() {
            None
        } else if std::env::var("ENOMOTO_XROW_AGGREGATOR").is_ok() {
            Some(timed_step!("aggregator", aggregator::eliminate_implied_free_columns_xrow(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs)))
        } else if std::env::var("ENOMOTO_ROWLOCAL_AGGREGATOR").is_ok() {
            // `None` = nothing eliminated (the problem is unchanged).
            timed_step!("aggregator", aggregator::eliminate_implied_free_columns_if_any(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs))
        } else {
            // Default since 2026-09-23 (analysis/stocfor2_presolve_20260923.md):
            // stocfor2 1652x1766 -> 950x1072 after presolve, -65% solve time.
            // `None` = no candidate at all (checked without copying the problem).
            timed_step!("aggregator", aggregator::eliminate_implied_free_columns_v2_if_any(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs, aggregator::AggOptions::from_env()))
        };
        if let Some(agg) = agg {
            if std::env::var("ENOMOTO_DEBUG_AGGREGATOR").is_ok() {
                eprintln!("DEBUG_AGGREGATOR: eliminated={}", agg.substitutions.len());
            }
            if !agg.substitutions.is_empty() {
                a = agg.a;
                b = agg.b;
                c = agg.c;
                for sub in &agg.substitutions {
                    lb[sub.var] = 0.0;
                    ub[sub.var] = 0.0;
                }
                postsolve_log.extend(agg.substitutions.into_iter().map(PostsolveStep::Sub));
                cur_real_rows = agg.real_rows;
                cur_real_rhs = agg.real_rhs;
                // Rebuild `g`/`h` from the just-updated `real_rows`/`lb`/`ub`
                // so the mid-round dedup call just below, and the next outer
                // round's own `propagate` call, see this pass's own changes
                // — same reason `doubleton`/`colsingleton` above must do
                // this (via the inner loop's own `rebuild_g` call) before
                // anything downstream reads `g`.
                let (ng, nh) = timed_step!("rebuild_g(agg)", propagate::rebuild_g_ref(n, &cur_real_rows, &cur_real_rhs, &lb, &ub));
                g = ng;
                h = nh;
            }
        }

        // ParallelColumns (see `parallelcols`'s own module docs): placed
        // right after `aggregator`, mirroring HiGHS's own log grouping of
        // "Aggregator" and "Parallel rows and columns" as adjacent passes
        // within the same outer main-loop iteration. Latched off after two
        // *consecutive* empty rounds (see `parallelcols_active`'s own docs
        // for why one strike isn't enough here, unlike
        // `doubleton_active`/`dualpropagate_active` above) — a 2026-09-20
        // survey of every one of the 73 in-scope Netlib instances found
        // only three (`ganges`, `czprob`, `greenbea`) with any nonzero-
        // elimination round after the first, so this latch skips the
        // (otherwise pure-overhead) repeat scan on most rounds of the
        // other 70.
        //
        // **Measured on the full 73-problem Netlib set (2026-09-20,
        // `ENOMOTO_ENABLE_PARALLELCOLS` A/B, before this latch existed):
        // net regression, +8.6% aggregate `ours` time (4.372s -> 4.746s),
        // concentrated on the same kind of degenerate/shape-sensitive
        // instances this crate's history keeps finding for *every*
        // structural presolve extension tried so far** (`aggregator`'s own
        // two earlier reverted attempts, the 6-technique bundle,
        // `parallelrows`/`dominatedcol`/`rowdominance`/`sparsify`/
        // `stuffing` — see each module's own docs): `wood1p` +33% (0
        // eliminations there, every round — pure candidate-search tax,
        // exactly what this latch now heads off), `scfxm3` +28%, `perold`
        // +22%, `maros` +23%, `pilotnov` +11%, `25fv47`/`degen3`/`stocfor2`
        // +4-10%. A follow-up 2026-09-21 measurement (after this latch, and
        // after the false-`Infeasible` fix below) found the same
        // instances' iteration counts don't uniformly increase — on
        // several (`nesm`/`scfxm3`/`ganges`) they actually *decrease* while
        // wall time still rises, because `XB_DRIFT_REL_TOL`-triggered
        // refactorizations increase 2-4x (see
        // `parallelcols-regression-mechanism` memory) — a real but so far
        // unaddressed cost, not a correctness concern.
        //
        // **Turned on by default anyway (2026-09-21)**, at the user's
        // explicit direction, to make forward progress on the actual
        // structural win (`standgub`'s 908 -> 830 columns, matching HiGHS's
        // easier exact-cost-ratio subset of its own 396-column "Parallel
        // rows and columns" reduction there) while leaving the refactor-
        // frequency regression above as deliberately deferred future work
        // — mirrors `aggregator`'s own `ENOMOTO_DISABLE_AGGREGATOR` opt-out
        // precedent instead of staying opt-in.
        //
        // Before flipping the default, a separate correctness bug was
        // found and fixed (see `parallelcols-greenbea-false-infeasible`
        // memory, and the merge loop's own comment in `parallelcols.rs`):
        // a merge with an opposite-signed leading coefficient could turn a
        // `kept` column genuinely free (`lb=-inf` *and* `ub=+inf`), which
        // `simplex.rs`'s own `x_j = x_j^+ - x_j^-` split handles
        // correctly on its own, but whose two split halves are forced onto
        // *exactly* the same rows with opposite coefficients — degenerate
        // enough on a real Netlib instance (`greenbea`, 5405 columns) to
        // exhaust `extended_dual`'s own `MAX_ITERS` budget, falling back to
        // the classical `BIG_M` path (already known unreliable — see
        // `bigm-fallback-invalid-reference` memory), which then reported a
        // false `Infeasible`. Fixed by rejecting that specific fold
        // outright rather than by touching anything downstream.
        let pc = if parallelcols_active && std::env::var("ENOMOTO_DISABLE_PARALLELCOLS").is_err() {
            // Inner `None` = ran, nothing merged (no copy of the problem made).
            Some(timed_step!("parallelcols", parallelcols::merge_parallel_columns_if_any(n, &a, &cur_real_rows, &c, &lb, &ub)))
        } else {
            None
        };
        if let Some(pc) = pc {
            if std::env::var("ENOMOTO_DEBUG_PARALLELCOLS").is_ok() {
                eprintln!("DEBUG_PARALLELCOLS: eliminated={}", pc.as_ref().map_or(0, |pc| pc.substitutions.len()));
            }
            if let Some(pc) = pc {
                parallelcols_empty_streak = 0;
                a = pc.a;
                c = pc.c;
                lb = pc.lb;
                ub = pc.ub;
                cur_real_rows = pc.real_rows;
                postsolve_log.extend(pc.substitutions.into_iter().map(PostsolveStep::ParallelCol));
                // Same reason as `aggregator`'s own rebuild just above: the
                // mid-round dedup call and the next round's `propagate`
                // must see this pass's own row/bound changes, not a stale
                // `g`/`h`.
                let (ng, nh) = timed_step!("rebuild_g(pc)", propagate::rebuild_g_ref(n, &cur_real_rows, &cur_real_rhs, &lb, &ub));
                g = ng;
                h = nh;
            } else {
                parallelcols_empty_streak += 1;
                if parallelcols_empty_streak >= tunable!("ENOMOTO_T_PARALLELCOLS_STRIKES", 2, usize) {
                    parallelcols_active = false;
                }
            }
        }

        // Re-run the cheap hash-based duplicate-row pass on `(g, h)` every
        // outer round, not just once before this loop starts (the original
        // design, mirroring `reduce_equalities`'s own one-shot placement) —
        // unlike that rank-revealing QR/Gaussian-elimination pass (expensive,
        // and gated to run once for that reason, matching HiGHS's own
        // `removeDependentEquations`), this one is a single O(nnz) hash scan,
        // and `doubleton`/`colsingleton` above rewrite `g`'s real inequality
        // rows during substitution (see either module's own `&g`/`&h` in its
        // signature) — so two originally-distinct inequality rows can end up
        // with the same coefficient pattern only *after* a shared variable is
        // eliminated from both, a duplicate this function's pre-loop call (on
        // the original, not-yet-substituted `g`) could never have seen. Run
        // unconditionally rather than latched (contrast `doubleton_active`):
        // cheap enough every round that gating it on a prior round finding
        // nothing isn't worth the extra bookkeeping.
        //
        // Measured directly (an `ENOMOTO_DEBUG_DEDUP_ROUND`-style row-count
        // counter, since removed, straight before/after this exact call) on
        // the full 73-problem in-scope Netlib set, A/B against this same
        // call sitting out here vs. only once before the round loop starts
        // (its pre-existing placement): this mid-loop call *does* fire —
        // on 44/73 instances, some substantially (`ganges` drops well over
        // 300 rows across its own several firings within one solve,
        // `sierra` over 150, `stocfor2`/`cycle` over 200 each) — unlike
        // `parallelrows`/`rowdominance`/`dominatedcol`/`sparsify`/`stuffing`
        // above and below, each of which fired on *zero* of these same 73.
        // Most of that mid-loop churn nets out to the same *final*
        // `g.nrows()` this function would have reached anyway (the same
        // duplicate row would otherwise have been resolved some other way
        // by a later `propagate`/`rowsingleton`/`colsingleton` pass instead)
        // — pure wasted work in every one of those later passes' own
        // per-round scans this call now heads off instead — except on two
        // instances where it also survives to reduce the truly *final*
        // post-presolve row count: `ganges` (948 -> 936, -1.3%) and
        // `sierra` (1097 -> 1087, -0.9%). Aggregate wall time across all 73
        // was a wash either way (10081.7ms without this call vs. 10091.2ms
        // with it, well within run-to-run noise) — kept for the two
        // instances' real size reduction and the wasted-work-avoided
        // argument above, not for any aggregate speedup this measurement
        // actually showed.
        let (rg, rh) = timed_step!("reduce_inequalities(round)", redundancy::reduce_inequalities(&g, &h, n));
        g = rg;
        h = rh;

        // `ENOMOTO_T_ROUND_STRUCT_STOP=1` (default 0 = off): stop as soon as
        // a whole round left the structure unchanged — `A`'s row count,
        // `g`'s multi-entry (real) row count, the number of fixed columns
        // and the postsolve log length all equal to the previous round's.
        // Unlike the signature check below this ignores bound values, so a
        // round that only keeps shaving bounds (geometric convergence) ends
        // the loop after one such idle round instead of running to the cap.
        if tunable!("ENOMOTO_T_ROUND_STRUCT_STOP", 0, usize) != 0 {
            let gr = g.as_ref();
            let g_multi = (0..gr.nrows()).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).count();
            let fixed = (0..n).filter(|&j| lb[j] == ub[j]).count();
            let st = (a.nrows(), g_multi, fixed, postsolve_log.len());
            if prev_struct == Some(st) {
                break;
            }
            prev_struct = Some(st);
        }

        let signature = (a.nrows(), g.nrows(), lb.clone(), ub.clone());
        // Bound changes below a relative 1e-3 do not count as progress
        // (`ENOMOTO_FIXPOINT_EXACT` restores the exact comparison):
        // propagation over a cyclic row structure — which the aggregator's
        // folds can create (`scagr25`) — keeps shaving ever-smaller amounts
        // off a bound (geometric convergence), which an exact comparison
        // never calls a fixpoint, burning every remaining round. The
        // tightened bounds themselves are still kept.
        let same = |p: &(usize, usize, Vec<f64>, Vec<f64>)| {
            if std::env::var("ENOMOTO_FIXPOINT_EXACT").is_ok() {
                return *p == signature;
            }
            let close = |x: &[f64], y: &[f64]| x.iter().zip(y).all(|(&u, &v)| u == v || (u - v).abs() <= tunable!("ENOMOTO_T_FIXPOINT_RELTOL", 1e-3, f64) * (1.0 + u.abs().max(v.abs())));
            p.0 == signature.0 && p.1 == signature.1 && close(&p.2, &signature.2) && close(&p.3, &signature.3)
        };
        if prev_signature.as_ref().is_some_and(same) {
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
            unbounded: false,
            postsolve_log,
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
    for step in &postsolve_log {
        if let PostsolveStep::Sub(sub) = step {
            lb[sub.var] = 0.0;
            ub[sub.var] = 0.0;
        }
    }

    // General free-variable elimination (paper §4.1): `rowsingleton`/
    // `doubleton`/`colsingleton` above only ever caught a free variable
    // appearing in exactly one or two `A` rows; this handles any remaining
    // one (any number of appearances, including none at all) — see
    // `freevar`'s own docs. Must run *after* the pinning loop just above,
    // not before: an already-eliminated column's own box row is long gone
    // by this point, so without pinning it to `[0,0]` first it would
    // misread here as a genuinely fresh free variable.
    let free = if std::env::var("ENOMOTO_DISABLE_FREEVAR").is_ok() {
        freevar::FreeVarResult {
            a: a.clone(),
            b: b.clone(),
            c: c.clone(),
            substitutions: Vec::new(),
            fixed: Vec::new(),
            unbounded: false,
            real_rows: prop.real_rows.clone(),
            real_rhs: prop.real_rhs.clone(),
        }
    } else {
        timed_step!("freevar", freevar::eliminate_free_variables(n, &a, &b, &c, &lb, &ub, &prop.real_rows, &prop.real_rhs))
    };
    if std::env::var("ENOMOTO_DEBUG_FREEVAR").is_ok() {
        eprintln!("DEBUG_FREEVAR: eliminated={} fixed={} unbounded={}", free.substitutions.len(), free.fixed.len(), free.unbounded);
    }
    if free.unbounded {
        return extended_unbounded(sc, free.a, free.b, free.c, n);
    }
    a = free.a;
    b = free.b;
    c = free.c;
    for &(j, v) in &free.fixed {
        lb[j] = v;
        ub[j] = v;
    }
    postsolve_log.extend(free.substitutions.into_iter().map(PostsolveStep::Sub));
    // Re-pin: covers `freevar`'s own newly eliminated columns (the pass
    // above only pinned what `rowsingleton`/`doubleton`/`colsingleton` had
    // already found) before `rebuild_g` reads `lb`/`ub` below — same reason
    // as the first pass, just for this pass's own new substitutions.
    for step in &postsolve_log {
        if let PostsolveStep::Sub(sub) = step {
            lb[sub.var] = 0.0;
            ub[sub.var] = 0.0;
        }
    }

    let (g, h) = propagate::rebuild_g_ref(n, &free.real_rows, &free.real_rhs, &lb, &ub);
    if std::env::var("ENOMOTO_DEBUG_PRESOLVE_HASH").is_ok() {
        eprintln!("PRESOLVE_HASH {:016x} m_eq={} m_le={} post={}", presolve_output_hash(&a, &b, &g, &h, &c, &lb, &ub), a.nrows(), g.nrows(), postsolve_log.len());
    }

    ExtendedPresolveResult {
        scaling: sc,
        a,
        b,
        g,
        h,
        lb,
        ub,
        real_rows: free.real_rows,
        real_rhs: free.real_rhs,
        c,
        infeasible: false,
        unbounded: false,
        postsolve_log,
    }
}

/// FNV-1a over the bit patterns of the reduced problem handed to the
/// simplex (`ENOMOTO_DEBUG_PRESOLVE_HASH`): two builds printing the same
/// hash hand the solver bit-identical input.
fn presolve_output_hash(a: &Csr, b: &[f64], g: &Csr, h: &[f64], c: &[f64], lb: &[f64], ub: &[f64]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        for byte in x.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for m in [a, g] {
        let r = m.as_ref();
        eat(r.nrows() as u64);
        eat(r.ncols() as u64);
        for i in 0..r.nrows() {
            let cols = r.col_indices_of_row_raw(i);
            eat(cols.len() as u64);
            for (&j, &v) in cols.iter().zip(r.values_of_row(i)) {
                eat(j as u64);
                eat(v.to_bits());
            }
        }
    }
    for v in [b, h, c, lb, ub] {
        eat(v.len() as u64);
        for &x in v {
            eat(x.to_bits());
        }
    }
    hash
}
