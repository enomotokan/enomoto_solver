import os, sys
os.chdir(sys.argv[1])
def patch(path, pairs):
    s = open(path).read()
    for old, new in pairs:
        assert s.count(old) == 1, (path, old[:70], s.count(old))
        s = s.replace(old, new)
    open(path, 'w').write(s)

patch('src/presolve.rs', [
("""    if !skip("reduce_eq") {
    let (na, nb) = timed_step!("reduce_equalities", redundancy::reduce_equalities(&a, &b, n, &pre_lb, &pre_ub));
    a = na;
    b = nb;
    }""", """    if !skip("reduce_eq") {
    let __p0 = a.nrows();
    let (na, nb) = timed_step!("reduce_equalities", redundancy::reduce_equalities(&a, &b, n, &pre_lb, &pre_ub));
    a = na;
    b = nb;
    if pipe_on { crate::pipeprof::count("red_eq_dropped", (__p0 - a.nrows()) as u64); crate::pipeprof::count("red_eq_rows_in", __p0 as u64); }
    }"""),
("""    if !skip("reduce_ineq0") {
    let (ng, nh) = timed_step!("reduce_inequalities", redundancy::reduce_inequalities(&g, &h, n));
    g = ng;
    h = nh;
    }""", """    if !skip("reduce_ineq0") {
    let __m0 = g.nrows();
    let (ng, nh) = timed_step!("reduce_inequalities", redundancy::reduce_inequalities(&g, &h, n));
    g = ng;
    h = nh;
    if pipe_on { crate::pipeprof::count("red_ineq0_dropped", (__m0 - g.nrows()) as u64); }
    }"""),
("""        let fixes = if skip("dualfix") { Vec::new() } else { timed_step!("dualfix", dualfix::fix_dominated_variables(n, &a, &cur_real_rows, &c, &lb, &ub)) };""",
 """        let fixes = if skip("dualfix") { Vec::new() } else { timed_step!("dualfix", dualfix::fix_dominated_variables(n, &a, &cur_real_rows, &c, &lb, &ub)) };
        if pipe_on { crate::pipeprof::count("dualfix_fixes", fixes.len() as u64); }
        let __dbg_rounds = std::env::var("ENOMOTO_PS_DEBUG_ROUNDS").is_ok();
        let __round_t0 = std::time::Instant::now();
        let __round_a0 = a.nrows(); let __round_g0 = cur_real_rows.len();
        let __round_fixed0 = (0..n).filter(|&j| lb[j] == ub[j]).count();"""),
("""            if dual_red.implied_equalities.is_empty() && dual_red.fixed_columns.is_empty() {
                dualpropagate_active = false;
            } else {""", """            if pipe_on { crate::pipeprof::count("dprop_eqs", dual_red.implied_equalities.len() as u64); crate::pipeprof::count("dprop_fixes", dual_red.fixed_columns.len() as u64); }
            if dual_red.implied_equalities.is_empty() && dual_red.fixed_columns.is_empty() {
                dualpropagate_active = false;
            } else {"""),
("""            for &(j, value) in &rs.fixes {
                lb[j] = value;
                ub[j] = value;
            }
            a = rs.a;""", """            if pipe_on { crate::pipeprof::count("rs_fixes", rs.fixes.len() as u64); }
            for &(j, value) in &rs.fixes {
                lb[j] = value;
                ub[j] = value;
            }
            a = rs.a;"""),
("""                if dbl.substitutions.is_empty() {
                    doubleton_active = false;
                }""", """                if pipe_on { crate::pipeprof::count("dbl_subs", dbl.substitutions.len() as u64); }
                if dbl.substitutions.is_empty() {
                    doubleton_active = false;
                }"""),
("""            a = cs.a;
            b = cs.b;
            c = cs.c;
            if !cs.substitutions.is_empty() {""", """            a = cs.a;
            b = cs.b;
            c = cs.c;
            if pipe_on { crate::pipeprof::count("cs_subs", cs.substitutions.len() as u64); }
            if !cs.substitutions.is_empty() {"""),
("""        if let Some(agg) = agg {
            if std::env::var("ENOMOTO_DEBUG_AGGREGATOR").is_ok() {""", """        if let Some(agg) = agg {
            if pipe_on { crate::pipeprof::count("agg_subs", agg.substitutions.len() as u64); if agg.substitutions.is_empty() { crate::pipeprof::count("agg_empty_calls", 1); } }
            if std::env::var("ENOMOTO_DEBUG_AGGREGATOR").is_ok() {"""),
("""        if let Some(pc) = pc {
            if std::env::var("ENOMOTO_DEBUG_PARALLELCOLS").is_ok() {""", """        if let Some(pc) = pc {
            if pipe_on { crate::pipeprof::count("pc_subs", pc.substitutions.len() as u64); if pc.substitutions.is_empty() { crate::pipeprof::count("pc_empty_calls", 1); } }
            if std::env::var("ENOMOTO_DEBUG_PARALLELCOLS").is_ok() {"""),
("""        if !skip("reduce_ineq_round") {
        let (rg, rh) = timed_step!("reduce_inequalities(round)", redundancy::reduce_inequalities(&g, &h, n));
        g = rg;
        h = rh;
        }
""", """        if !skip("reduce_ineq_round") {
        let __m0 = g.nrows();
        let (rg, rh) = timed_step!("reduce_inequalities(round)", redundancy::reduce_inequalities(&g, &h, n));
        g = rg;
        h = rh;
        if pipe_on { crate::pipeprof::count("red_ineq_round_dropped", (__m0 - g.nrows()) as u64); }
        }
        if __dbg_rounds {
            let fixed_now = (0..n).filter(|&j| lb[j] == ub[j]).count();
            let (_, _, rr, _) = propagate::extract_bounds(n, &g, &h);
            eprintln!("PS_ROUND {} a_rows {}->{} g_real {}->{} fixed {}->{} postsolve_log={} t={:.1}us", _round_idx, __round_a0, a.nrows(), __round_g0, rr.len(), __round_fixed0, fixed_now, postsolve_log.len(), __round_t0.elapsed().as_secs_f64() * 1e6);
        }
"""),
("""    postsolve_log.extend(free.substitutions.into_iter().map(PostsolveStep::Sub));
    // Re-pin:""", """    if pipe_on { crate::pipeprof::count("free_subs", free.substitutions.len() as u64); crate::pipeprof::count("free_fixed", free.fixed.len() as u64); }
    postsolve_log.extend(free.substitutions.into_iter().map(PostsolveStep::Sub));
    // Re-pin:"""),
])

patch('src/presolve/redundancy.rs', [
("""    let keep = if density > DENSE_DENSITY_THRESHOLD {
        drop_linearly_dependent(&deduped, n)
    } else {""", """    let dense_thr: f64 = std::env::var("ENOMOTO_PS_DENSE_THRESHOLD").ok().and_then(|s| s.parse().ok()).unwrap_or(DENSE_DENSITY_THRESHOLD);
    if crate::pipeprof::enabled() { crate::pipeprof::count("red_eq_dedup_dropped", (p - deduped.len()) as u64); crate::pipeprof::count(if density > dense_thr { "red_eq_dense" } else { "red_eq_sparse" }, 1); }
    let keep = if density > dense_thr {
        drop_linearly_dependent(&deduped, n)
    } else {"""),
("""    if rows_in.len() < MIN_ROWS_FOR_BLOCK_DECOMPOSE {
        return drop_linearly_dependent_sparse(rows_in, n);
    }
    let components = dulmage_mendelsohn_blocks(rows_in, n, lb, ub);""", """    let min_block: usize = std::env::var("ENOMOTO_PS_MIN_BLOCK_ROWS").ok().and_then(|s| s.parse().ok()).unwrap_or(MIN_ROWS_FOR_BLOCK_DECOMPOSE);
    if rows_in.len() < min_block {
        return drop_linearly_dependent_sparse(rows_in, n);
    }
    let __t_dm = std::time::Instant::now();
    let components = dulmage_mendelsohn_blocks(rows_in, n, lb, ub);
    if crate::pipeprof::enabled() { crate::pipeprof::add("red_eq_dm", __t_dm); crate::pipeprof::count("red_eq_dm_components", components.len() as u64); }"""),
("""    if nontrivial.len() > 1 && total_nontrivial_rows >= PARALLEL_DECOMPOSE_ROW_THRESHOLD {
        use rayon::prelude::*;""", """    if nontrivial.len() > 1 && total_nontrivial_rows >= PARALLEL_DECOMPOSE_ROW_THRESHOLD && std::env::var("ENOMOTO_PS_NO_RAYON").is_err() {
        if crate::pipeprof::enabled() { crate::pipeprof::count("red_eq_rayon", 1); }
        use rayon::prelude::*;"""),
])

patch('src/simplex.rs', [
("""    if pipe_on { crate::pipeprof::add("build_std_form_presolved", __t); }
""", """    if pipe_on { crate::pipeprof::add("build_std_form_presolved", __t); crate::pipeprof::count("n_in", variables.len() as u64); crate::pipeprof::count("m_in", constraints.len() as u64); crate::pipeprof::count("n_out", (std.n_total - std.n_rows) as u64); crate::pipeprof::count("m_out", std.n_rows as u64); crate::pipeprof::count("post_len", postsolve_log.len() as u64); }
"""),
])
print("patch3 ok")
