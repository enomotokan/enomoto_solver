//! 診断 `ENOMOTO_MIP_SYMPRE0_DIAG` (analysis/sympre0_20261111.md、求解は変えない): 前処理の前 (DRCR で畳んだ後) と前処理の後の
//! 問題に、今の検出 (`add_symmetry_rows` と同じ写し・同じ上限) をかけ、置換の対称性の生成元・動く列・双子・時間を並べる。
//! 値に `x10` などの `x<倍率>` を含めば、手間と時間の上限をその倍にする (上限で見落とした分を見る)。

use super::problem::MipProblem;
use super::symmetry;

fn cap_mult() -> f64 {
    env_str!("ENOMOTO_MIP_SYMPRE0_DIAG")
        .and_then(|v| v.split('x').nth(1).and_then(|s| s.trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.').parse::<f64>().ok()))
        .unwrap_or(1.0)
}

fn find(u: &mut [usize], mut a: usize) -> usize {
    while u[a] != a {
        u[a] = u[u[a]];
        a = u[a];
    }
    a
}

/// `p` に今の検出をかけて 1 行で出す (`label` は "before"・"after")。生成元を返す。
pub fn diag(p: &MipProblem, label: &str) -> Vec<Vec<usize>> {
    let t0 = std::time::Instant::now();
    let twins = symmetry::twin_classes(p);
    let k = cap_mult();
    // `add_symmetry_rows` の既定 (`ENOMOTO_MIP_PRESOLVE_WORK`・`HO3_SYMNORM`・`SYMSIGN`) と同じ上限と写し
    let (mut time_cap, mut caps) = (tunable!("ENOMOTO_T_MIP_SYM_TIME", 0.3, f64), (u64::MAX, u64::MAX));
    if super::presolve_work() {
        time_cap *= super::presolve_work_safe();
        caps = (tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_SYM", 2.5e7, f64) as u64, tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG", 1.3e8, f64) as u64);
    }
    let (q, [_, _, ties]) = symmetry::detection_copy(p, false, true);
    let mut r = k;
    if ties > 0 {
        r *= (q.nnz + q.m) as f64 / (p.nnz + p.m).max(1) as f64;
    }
    let sc = |c: u64| if c == u64::MAX { c } else { ((c as f64) * r).min(u64::MAX as f64) as u64 };
    caps = (sc(caps.0), sc(caps.1));
    time_cap *= r;
    let (sym, _) = super::sympre::detect(&q, time_cap, tunable!("ENOMOTO_T_MIP_SYM_TRIES", 2000usize, usize), caps.0, caps.1, &twins, false);
    let secs = t0.elapsed().as_secs_f64();
    let gens: Vec<Vec<usize>> = sym.map(|s| s.generators).unwrap_or_default();
    let n = p.n;
    let mut u: Vec<usize> = (0..n).collect();
    let mut moved = vec![false; n];
    for g in &gens {
        for j in 0..n {
            if g[j] != j {
                moved[j] = true;
                let (a, b) = (find(&mut u, j), find(&mut u, g[j]));
                u[a] = b;
            }
        }
    }
    let gen_cols = moved.iter().filter(|&&b| b).count();
    let twin_cols: usize = twins.iter().map(|t| t.len()).sum();
    for t in &twins {
        for w in t.windows(2) {
            let (a, b) = (find(&mut u, w[0]), find(&mut u, w[1]));
            u[a] = b;
        }
    }
    let mut size = vec![0usize; n];
    for j in 0..n {
        let r = find(&mut u, j);
        size[r] += 1;
    }
    let (orb, orb_cols) = size.iter().filter(|&&s| s > 1).fold((0, 0), |(a, b), &s| (a + 1, b + s));
    let ints = (0..n).filter(|&j| size[find(&mut u, j)] > 1 && p.is_int[j]).count();
    eprintln!(
        "MIP: SYMPRE0 {label}: n {} m {} nnz {}; generators {}, moved {gen_cols} columns; twins {} classes {twin_cols} columns; orbits {orb} on {orb_cols} columns ({ints} integer) ({secs:.3}s, caps x{k})",
        p.n,
        p.m,
        p.nnz,
        gens.len(),
        twins.len()
    );
    gens
}
