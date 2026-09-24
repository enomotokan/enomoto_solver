import os, sys
os.chdir(sys.argv[1])
def patch(path, pairs):
    s = open(path).read()
    for old, new in pairs:
        assert s.count(old) == 1, (path, old[:70], s.count(old))
        s = s.replace(old, new)
    open(path, 'w').write(s)

# (a) reduce_equalities: dedupe only (skip rank step) via ENOMOTO_PS_SKIP=red_eq_rank ; also fast dedupe variant
patch('src/presolve/redundancy.rs', [
("""    let deduped = dedupe_rows(rows);
    let nnz: usize = deduped.iter().map(|(row, _)| row.len()).sum();""",
 """    let deduped = dedupe_rows(rows);
    if std::env::var("ENOMOTO_PS_SKIP").map_or(false, |s| s.split(',').any(|x| x == "red_eq_rank")) {
        let rows_out: Vec<Vec<(usize, f64)>> = deduped.iter().map(|(r, _)| r.clone()).collect();
        return (csr_from_rows(&rows_out, n), deduped.iter().map(|(_, b)| *b).collect());
    }
    let nnz: usize = deduped.iter().map(|(row, _)| row.len()).sum();"""),
# (b) reduce_inequalities: skip unit rows from hashing (ENOMOTO_PS_INEQ_SKIP_UNIT)
("""    let mut keep = vec![true; m];
    let mut any_dropped = false;
    for idx in 0..m {
        let cols = gr.col_indices_of_row_raw(idx);
        let vals = gr.values_of_row(idx);
        if cols.is_empty() {
            continue;
        }""", """    let mut keep = vec![true; m];
    let mut any_dropped = false;
    let skip_unit = std::env::var("ENOMOTO_PS_INEQ_SKIP_UNIT").is_ok();
    if std::env::var("ENOMOTO_PS_INEQ_CAP").is_ok() { heads.reserve(m); }
    for idx in 0..m {
        let cols = gr.col_indices_of_row_raw(idx);
        let vals = gr.values_of_row(idx);
        if cols.is_empty() || (skip_unit && cols.len() == 1) {
            continue;
        }"""),
])

# (c) aggregator v2: cheap candidate pre-check, call v2 only when a candidate exists (ENOMOTO_PS_AGG_PRECHECK)
patch('src/presolve/aggregator.rs', [
("""pub fn eliminate_implied_free_columns_v2(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> AggregatorResult {
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);""",
 """pub fn v2_has_candidate(n: usize, a: &Csr, b: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> bool {
    let ar = a.as_ref();
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;
    let mut col_a_count = vec![0usize; n];
    let mut col_g_count = vec![0usize; n];
    for i in 0..ar.nrows() {
        for (&j, &v) in ar.col_indices_of_row_raw(i).iter().zip(ar.values_of_row(i)) {
            if v != 0.0 { col_a_count[j] += 1; }
        }
    }
    for row in real_rows {
        for &(j, v) in row {
            if v != 0.0 { col_g_count[j] += 1; }
        }
    }
    let min_a = opts.min_a_count.max(1);
    let mut act_a: Vec<Option<RowActivity>> = vec![None; ar.nrows()];
    let mut act_g: Vec<Option<RowActivity>> = vec![None; real_rows.len()];
    let mut lo = vec![f64::NEG_INFINITY; n];
    let mut hi = vec![f64::INFINITY; n];
    let mut eligible = vec![false; n];
    for j in 0..n {
        eligible[j] = !(col_a_count[j] < min_a || col_a_count[j] + col_g_count[j] < 2 || is_free(j));
    }
    let mut row_buf: Vec<(usize, f64)> = Vec::new();
    for i in 0..ar.nrows() {
        row_buf.clear();
        row_buf.extend(ar.col_indices_of_row_raw(i).iter().zip(ar.values_of_row(i)).map(|(&j, &v)| (j, v)));
        if !row_buf.iter().any(|&(j, v)| v != 0.0 && eligible[j]) { continue; }
        let act = *act_a[i].get_or_insert_with(|| compute_row_activity(&row_buf, lb, ub));
        for &(j, coeff) in row_buf.iter() {
            if coeff == 0.0 || !eligible[j] { continue; }
            let (rlo, rhi) = implied_range(&act, j, coeff, b[i], lb, ub);
            lo[j] = lo[j].max(rlo);
            hi[j] = hi[j].min(rhi);
        }
    }
    if opts.use_ineq {
        for (i, row) in real_rows.iter().enumerate() {
            if !row.iter().any(|&(j, v)| v != 0.0 && eligible[j]) { continue; }
            let act = *act_g[i].get_or_insert_with(|| compute_row_activity(row, lb, ub));
            for &(j, coeff) in row.iter() {
                if coeff == 0.0 || !eligible[j] { continue; }
                let (s_lo, _) = residual_range(&act, j, coeff, lb, ub);
                if s_lo.is_finite() {
                    let v = (real_rhs[i] - s_lo) / coeff;
                    if coeff > 0.0 { hi[j] = hi[j].min(v); } else { lo[j] = lo[j].max(v); }
                }
            }
        }
    }
    (0..n).any(|j| eligible[j] && range_within_box(j, lo[j], hi[j], lb, ub))
}

pub fn eliminate_implied_free_columns_v2(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> AggregatorResult {
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);"""),
])
patch('src/presolve.rs', [
("""            Some(timed_step!("aggregator", aggregator::eliminate_implied_free_columns_v2(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs, aggregator::AggOptions::from_env())))""",
 """            if std::env::var("ENOMOTO_PS_AGG_PRECHECK").is_ok() {
                let opts = aggregator::AggOptions::from_env();
                let __t = std::time::Instant::now();
                let has = aggregator::v2_has_candidate(n, &a, &b, &lb, &ub, &cur_real_rows, &cur_real_rhs, opts);
                if pipe_on { crate::pipeprof::add("agg_precheck", __t); }
                if has { Some(timed_step!("aggregator", aggregator::eliminate_implied_free_columns_v2(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs, opts))) } else { if pipe_on { crate::pipeprof::count("agg_precheck_skip", 1); } None }
            } else {
            Some(timed_step!("aggregator", aggregator::eliminate_implied_free_columns_v2(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs, aggregator::AggOptions::from_env())))
            }"""),
])
print("patch4 ok")
