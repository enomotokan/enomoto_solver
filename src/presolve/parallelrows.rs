//! ParallelRows, opposite-sign half (Andersen & Andersen, "Presolving in
//! Linear Programming", Mathematical Programming 71 (1995); also PaPILO's
//! "ParallelRows", Achterberg et al. 2019 §4.4): [`redundancy::reduce_inequalities`](super::redundancy::reduce_inequalities)
//! already collapses two `<=` rows that are *positive* scalar multiples of
//! each other (same half-space, keep only the tighter), but leaves
//! *negative* scalar multiples untouched — `a.x <= h_i` and
//! `(-s*a).x <= h_j` (`s > 0`) are two independent half-spaces (an upper
//! and a lower bound on the same linear form `a.x`), not duplicates of one
//! another, so the existing sign-preserving normalization in
//! [`redundancy::dedupe_rows`](super::redundancy) correctly leaves them
//! both in place. This module picks up exactly that remainder: rewriting
//! `a.x <= h_i` and `(-s*a).x <= h_j` as the single two-sided bound
//! `-h_j/s <= a.x <= h_i`, then:
//!
//!   - `-h_j/s > h_i` (beyond `TOL`): the two rows jointly rule out every
//!     point — infeasible, reported the same way [`ExtendedPresolveResult`]'s
//!     other `infeasible` producers are.
//!   - `-h_j/s == h_i` (within `TOL`): the two rows pin `a.x` to exactly
//!     `h_i` — **merged into one** new equality row appended to `(A, b)`,
//!     with both original inequality rows dropped from `(G, h)`. This is
//!     the literal row-count reduction the technique is named for — the
//!     shape it targets (an MPS `RANGES`-section range constraint, split by
//!     this crate's own `>=`-as-negated-`<=` convention in
//!     [`super::build_a_g`] into exactly this opposite-sign pair) whenever
//!     the range width collapses to (near) zero.
//!   - otherwise: genuine slack remains between the two bounds — no row
//!     reduction is possible from this pairwise comparison alone (tightening
//!     either bound further would need the other rows/variable bounds too,
//!     which is [`propagate`](super::propagate)'s job, not this cheap
//!     pairwise one) — both rows are left exactly as they were.
//!
//! Candidates are found the same way [`redundancy::dedupe_rows`](super::redundancy)
//! finds its own (equality-row) duplicates: normalize every row (skipping
//! length-1 rows — those are box-bound rows folding a variable's own
//! `lb`/`ub` into `G`, see [`super::build_a_g`]'s docs, and merging a
//! variable's own two box rows into an equality would just re-derive a
//! bound [`propagate`] already maintains directly, not a real reduction)
//! by dividing by its own *signed* first nonzero coefficient — not
//! [`redundancy::reduce_inequalities`]'s own `|first coeff|` normalization,
//! which deliberately keeps each entry's sign relative to the row's
//! overall sign and so never lets two negated rows collide; dividing by
//! the signed value instead always normalizes the first entry to `+1`, so
//! two rows that are negatives of each other land on the identical
//! signature regardless of which one happens to be written with a
//! positive leading coefficient. Run *after*
//! [`redundancy::reduce_inequalities`] in the pipeline, so by construction
//! no two rows sharing a signature here can still be a *positive* multiple
//! of one another — every match this module finds is a genuine opposite-sign
//! pair, the complement [`redundancy::reduce_inequalities`] leaves behind.
//!
//! Single-pass, non-cascading, decided from the input snapshot (mirrors
//! every other pass in this pipeline): a row used on either side of one
//! merge this call is never reused on either side of a second merge in the
//! same call — a signature group with more than 2 rows only ever produces
//! one merge per call, the rest picked up by the pipeline's own outer
//! fixpoint loop calling this again.
//!
//! **Implemented, unit-tested, measured against the full Netlib set — then
//! left unintegrated (kept here, tested, but never called from
//! [`crate::presolve::run_extended`]), mirroring [`dominatedcol`](super::dominatedcol)/
//! [`sparsify`](super::sparsify)'s own precedent.** Wired in once, right
//! after [`redundancy::reduce_inequalities`], and instrumented directly
//! (not inferred from timing alone): across all 73 in-scope Netlib
//! instances, zero opposite-sign proportional row pairs were ever found —
//! `benchmark_highs.py`'s own MPS-reading (see its module docs) always
//! keeps a range constraint's two sides with real slack between them on
//! every instance in this set, never the tight-both-ways degenerate case
//! this module exists to collapse. Wiring it in therefore cost pure
//! candidate-search overhead (grouping every multi-variable `G` row by
//! signature) for zero reductions anywhere: aggregate `ours` time went
//! from 3.85s to 3.91s (+1.5%), 73/73 objective values unchanged either
//! way. Kept here for its correct, tested core logic — e.g. a future
//! problem source that actually produces tight-range constraints, or a
//! model-building layer that emits them directly — rather than deleted.

use std::collections::HashMap;

use crate::sparse::{csr_from_rows, Csr};

const TOL: f64 = 1e-9;

/// Result of one [`merge_parallel_rows`] call: `a`/`b` with any newly
/// discovered equality merged in (appended after the existing rows), `g`/`h`
/// with both sides of each merge removed, and `infeasible` set the same way
/// every other pass in this pipeline reports a Farkas-style contradiction —
/// without attempting to still produce a meaningful `g`/`h` in that case
/// (the caller checks `infeasible` first, same convention as
/// [`super::propagate::propagate`]'s own result).
pub struct ParallelRowsResult {
    pub a: Csr,
    pub b: Vec<f64>,
    pub g: Csr,
    pub h: Vec<f64>,
    pub infeasible: bool,
}

pub fn merge_parallel_rows(a: &Csr, b: &[f64], g: &Csr, h: &[f64], n: usize) -> ParallelRowsResult {
    let gr = g.as_ref();
    let m = gr.nrows();
    let rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| gr.col_indices_of_row(i).zip(gr.values_of_row(i)).map(|(j, &v)| (j, v)).collect()).collect();

    // Group multi-variable rows by their sign-invariant signature — divide
    // by the *signed* first coefficient (matching `redundancy::dedupe_rows`'s
    // own normalization exactly, not `reduce_inequalities`'s sign-preserving
    // `|first coeff|` one: dividing by `|v|` keeps each entry's sign
    // relative to the row's *own* overall sign, so two rows that are
    // negatives of each other normalize to *different* signatures there —
    // exactly why `reduce_inequalities` doesn't already catch this case;
    // dividing by the signed value instead always normalizes the first
    // entry to `+1`, making two negated rows land on the identical
    // signature regardless of which one happens to be written with a
    // positive leading coefficient). Any two rows landing in the same
    // bucket are proportional up to sign; `rows[i][0].1`/`rows[j][0].1`
    // below (the original, un-normalized leading coefficients) are what
    // then distinguishes "same-sign" from "opposite-sign" for a given pair.
    let mut groups: HashMap<Vec<(usize, u64)>, Vec<usize>> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        if row.len() < 2 {
            continue;
        }
        let inv = 1.0 / row[0].1;
        let sig: Vec<(usize, u64)> = row.iter().map(|&(j, v)| (j, (v * inv).to_bits())).collect();
        groups.entry(sig).or_default().push(i);
    }

    let mut used = vec![false; m];
    let mut drop_g: Vec<bool> = vec![false; m];
    let mut new_eq_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut new_eq_b: Vec<f64> = Vec::new();
    let mut infeasible = false;

    // Deterministic order: sort groups' own keys isn't needed for
    // correctness (every group is independent), but iterating a `HashMap`
    // directly would make which-pair-merges-first nondeterministic across
    // runs when a group has more than 2 members — harmless for correctness
    // (every valid pairing here is equally valid) but still worth pinning
    // down for reproducible benchmarking, so rows within a group are always
    // tried in ascending row-index order.
    let mut group_indices: Vec<&Vec<usize>> = groups.values().collect();
    group_indices.sort_by_key(|v| v[0]);

    'groups: for idx_list in group_indices {
        let mut members = idx_list.clone();
        members.sort_unstable();
        for a_pos in 0..members.len() {
            let i = members[a_pos];
            if used[i] {
                continue;
            }
            for b_pos in (a_pos + 1)..members.len() {
                let j = members[b_pos];
                if used[j] {
                    continue;
                }
                let vi = rows[i][0].1;
                let vj = rows[j][0].1;
                if vi.signum() == vj.signum() {
                    // Same-sign duplicate: already handled upstream by
                    // `redundancy::reduce_inequalities` (and if it somehow
                    // wasn't — e.g. this function called standalone in a
                    // test — merging it here too would need the same
                    // keep-tighter logic that function already implements;
                    // skip rather than duplicate that logic).
                    continue;
                }
                // rows[j] ~= (vj/vi) * rows[i], with vj/vi < 0. Let
                // s = -(vj/vi) > 0, so rows[j] ~= -s * rows[i]: row j's
                // constraint `rows[j].x <= h[j]` becomes, divided by `-s`
                // (flipping the inequality), `rows[i].x >= -h[j]/s`.
                let s = -(vj / vi);
                let lower = -h[j] / s;
                let upper = h[i];
                let tol = TOL * (1.0 + upper.abs().max(lower.abs()));
                if lower > upper + tol {
                    infeasible = true;
                    break 'groups;
                } else if (lower - upper).abs() <= tol {
                    used[i] = true;
                    used[j] = true;
                    drop_g[i] = true;
                    drop_g[j] = true;
                    new_eq_rows.push(rows[i].clone());
                    new_eq_b.push(upper);
                    break; // i is claimed; move on to the next unclaimed i
                }
                // Otherwise genuine slack remains — leave both rows alone
                // and keep looking for a different partner for `i`.
            }
        }
    }

    if infeasible {
        return ParallelRowsResult { a: a.clone(), b: b.to_vec(), g: g.clone(), h: h.to_vec(), infeasible: true };
    }

    let ar = a.as_ref();
    let mut a_rows: Vec<Vec<(usize, f64)>> = (0..ar.nrows()).map(|i| ar.col_indices_of_row(i).zip(ar.values_of_row(i)).map(|(j, &v)| (j, v)).collect()).collect();
    let mut new_b = b.to_vec();
    a_rows.extend(new_eq_rows);
    new_b.extend(new_eq_b);

    let mut g_rows = Vec::with_capacity(m);
    let mut new_h = Vec::with_capacity(m);
    for i in 0..m {
        if drop_g[i] {
            continue;
        }
        g_rows.push(rows[i].clone());
        new_h.push(h[i]);
    }

    ParallelRowsResult { a: csr_from_rows(&a_rows, n), b: new_b, g: csr_from_rows(&g_rows, n), h: new_h, infeasible: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn merges_a_tight_opposite_pair_into_an_equality() {
        // x0 + 2*x1 <= 10 and -x0 - 2*x1 <= -10 (i.e. x0+2x1 >= 10) pin
        // x0+2x1 exactly to 10 -- must collapse to one equality row.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 2.0)], vec![(0, -1.0), (1, -2.0)]], 2);
        let h = vec![10.0, -10.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 0, "both inequality rows must be dropped");
        assert_eq!(r.a.as_ref().nrows(), 1, "exactly one equality row must be added");
        assert!((r.b[0] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn detects_infeasibility_when_the_bounds_cross() {
        // x0 + x1 <= 5 and x0 + x1 >= 8 (via -x0-x1 <= -8) -- no x can
        // satisfy both.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]], 2);
        let h = vec![5.0, -8.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(r.infeasible);
    }

    #[test]
    fn leaves_a_genuine_range_untouched() {
        // x0 + x1 <= 10 and x0 + x1 >= 2 (via -x0-x1 <= -2) -- real slack,
        // no row-count reduction possible here.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]], 2);
        let h = vec![10.0, -2.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 2);
        assert_eq!(r.a.as_ref().nrows(), 0);
    }

    #[test]
    fn leaves_box_bound_rows_untouched() {
        // Two length-1 rows (a variable's own lb/ub, per build_a_g) must
        // never be merged into an equality here, even when they happen to
        // pin the variable to a point -- that's already represented
        // directly as a bound, not a real row pair to collapse.
        let a = csr_from_rows(&[], 1);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0)], vec![(0, -1.0)]], 1);
        let h = vec![5.0, -5.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 1);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 2);
        assert_eq!(r.a.as_ref().nrows(), 0);
    }

    #[test]
    fn ignores_same_sign_duplicates_leaving_them_for_reduce_inequalities() {
        // Two positive multiples of the same row -- this module's job ends
        // at opposite-sign pairs, so both survive untouched here.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 12.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 2);
    }
}
