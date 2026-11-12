//! `ENOMOTO_MIP_FOLD` (2026-11-10 から既定で使う、既定の値 `riq`。`0` で止める。analysis/drcr_20261110.md): MIP を色の細分化で畳んで解き、解を元に戻す
//! (van der Hulst 2026「Folding Mixed-Integer Linear Programs and Reflection Symmetries」の第 4 節)。畳み方の本体は
//! [`crate::simplex::lp_fold`]。
//!
//! 値の文字: `r` 反転も使う、`i` 整数列もまとめる (段階 3)、`n` 整数の組の TU の判定にネットワーク行列 (とその転置) の一般の判定も使う
//! (試作、既定では使わない。analysis/drcrtu_20261111.md)、`q` ログを出さない、`c` 畳んだ数を出すだけで解かない (数える用)、
//! `p` 行が 1 本もまとまらない (平行な列だけの) 畳みと、列が 1 本もまとまらない (平行な行だけの) 畳みも使う (既定では使わない。
//! 前処理が同じ列・同じ行をまとめるので)、`P` 前処理 → 畳む・`Q` 前処理 → 畳む → 前処理 (試作、[`solve_folded_any`])。`1` は連続列だけ・反転なし。
//!
//! - 連続列だけ (段階 2): 整数列はどれも 1 列だけの組にする (論文の Lemma 28)。和の等しい行は整数列の係数が同じになるので、
//!   畳んだ MIP と元の MIP は解が行き来でき (x_w = δ_w + λ_w z_Q、整数列は δ が整数)、最適値も同じ。
//! - 整数列も (段階 3、論文の Algorithm 1 を簡単にしたもの): 整数の組 Q は和の変数 y_Q = Σ λ_w (x_w − δ_w) (整数) にまとめる。
//!   畳んだ MIP は元の緩和 (元の整数解は畳んだ問題の整数解に写り目的値が同じ、Theorem 26 (i)) なので下界はそのまま使える。
//!   元に戻すには、整数の 1 列の組の値と整数の組の和を固定した元の問題 (繊維) の LP を解き、頂点が整数なら元の整数解
//!   (Theorem 32)。繊維が整数の多面体になるよう、組ごとに「組の列の係数が行の中で λ に比例する行は打ち消す (アフィンな TU の
//!   分解 A = S + U T の U)、残り S と組の和の行 T を並べた行列が TU」を確かめた組だけまとめる。TU の判定はネットワーク行列の
//!   一般の判定ではなく、剥がし (非零 1 つ以下の行・列を除く) と Heller-Tompkins の 2 色塗り (列の非零が 2 つ、またはその転置) の
//!   十分条件だけ。連続列を含む行と、係数・左辺・右辺が整数でない行 (V_R) では組の列が全部打ち消されることを求める
//!   (連続列の部分の TU は使わない。論文より狭い)。
//! - 安全網: 戻した解は元の問題で実行可能か (整数性も) を全部の回で確かめ、段階 3 では目的値が畳んだ問題の値以下 (最適の証明)
//!   かも確かめる。外れたら畳まずに残りの時間で元の問題を解く。

use super::problem::MipProblem;
use super::solver::{MipResult, MipStatus};
use crate::simplex::lp_fold::{Fold, FoldIn, FoldOpts};

/// `ENOMOTO_MIP_FOLD` の値。2026-11-10 から既定で使う (既定の値 `riq` = 反転・整数の組・ログなし。`0` か `off` で止める。
/// analysis/drcr_20261110.md の既定化の節)。
pub fn flags() -> Option<&'static str> {
    env_def!("ENOMOTO_MIP_FOLD", "riq")
}

/// 畳み方と畳んだ問題。
pub struct MipFold {
    pub fold: Fold,
    pub q: MipProblem,
    /// 整数の組 (2 列以上) があるか (戻すのに繊維の LP が要る)。
    pub int_groups: bool,
    pub secs: f64,
}

fn integral(v: f64) -> bool {
    (v - v.round()).abs() <= 1e-9 * (1.0 + v.abs())
}

fn fold_in<'a>(p: &'a MipProblem, int_key: Option<&'a [u64]>, single: Option<&'a [bool]>) -> FoldIn<'a> {
    FoldIn { col_lo: &p.col_lo, col_up: &p.col_up, cost: &p.cost, is_int: &p.is_int, rows: &p.rows, row_lo: &p.row_lo, row_up: &p.row_up, int_key, single }
}

/// 段階 3 の整数列の初めの鍵: V_R の行の係数 (符号を揃えて) と、ほかの行の ±1 でない係数の位置と値。
fn int_keys(p: &MipProblem, vr: &[bool]) -> Vec<u64> {
    use std::hash::{Hash, Hasher};
    (0..p.n)
        .map(|j| {
            if !p.is_int[j] {
                return 0;
            }
            let mut ent: Vec<(usize, i64, bool)> = Vec::new();
            for &(i, a) in &p.cols[j] {
                if vr[i] {
                    ent.push((i, (a * 1e9).round() as i64, true));
                } else if a.abs() != 1.0 {
                    ent.push((i, (a * 1e9).round() as i64, false));
                }
            }
            ent.sort_unstable();
            if ent.first().is_some_and(|e| e.1 < 0) {
                for e in ent.iter_mut() {
                    e.1 = -e.1;
                }
            }
            let mut h = std::collections::hash_map::DefaultHasher::new();
            ent.hash(&mut h);
            h.finish()
        })
        .collect()
}

/// 畳む。畳まないなら `None`。
pub fn try_fold(p: &MipProblem, verbose: bool) -> Option<MipFold> {
    try_fold_with(p, verbose, flags()?)
}

/// [`try_fold`] の本体 (値の文字 `fl` を引数で受け取る。数える用の [`count_hook`] からも呼ぶ)。
pub fn try_fold_with(p: &MipProblem, verbose: bool, fl: &str) -> Option<MipFold> {
    let log = verbose || !fl.contains('q');
    let t0 = std::time::Instant::now();
    let opts = crate::simplex::lp_fold::opts_from_env(fl.contains('r'));
    let ints = fl.contains('i');
    let f = if !ints {
        Fold::build(&fold_in(p, None, None), opts)
    } else {
        // V_R: 連続列を含む行、係数か有限の左辺・右辺が整数でない行
        let vr: Vec<bool> = (0..p.m)
            .map(|i| {
                p.rows[i].iter().any(|&(j, a)| !p.is_int[j] || !integral(a))
                    || (p.row_lo[i].is_finite() && !integral(p.row_lo[i]))
                    || (p.row_up[i].is_finite() && !integral(p.row_up[i]))
            })
            .collect();
        let keys = int_keys(p, &vr);
        if env_str!("ENOMOTO_T_FOLD_PROF").is_some() { eprintln!("FOLD_PROF mip keys {:.3}s", t0.elapsed().as_secs_f64()); }
        let f1 = Fold::build(&fold_in(p, Some(&keys), None), FoldOpts { max_ratio: 1.0, ..opts });
        match f1 {
            None => None,
            Some(f1) => {
                if env_str!("ENOMOTO_T_FOLD_PROF").is_some() { eprintln!("FOLD_PROF mip f1 {:.3}s", t0.elapsed().as_secs_f64()); }
                // 整数の組ごとに打ち消しと TU を確かめる
                let mut single = vec![false; p.n];
                // TU を確かめる行列 (列 → (行, 係数)) を行の連結成分ごとに持つ (行 0..m は元の行、m.. は組の和の行)。TU の十分条件
                // (`tu::tu_cols_f64`: 剥がしと芯の 2 色塗り) は連結成分ごとに閉じているので、新しい組が触れる成分だけを集めて確かめれば、
                // 全体を確かめるのと同じ答えになる (全体を毎回複写して確かめると組の数の 2 乗の手間になり、neos-631710 で 208 秒かかった)
                let mut nrows_m = p.m;
                let mut dsu: Vec<usize> = (0..p.m).collect();
                let mut comp_cols: std::collections::HashMap<usize, Vec<Vec<(usize, f64)>>> = std::collections::HashMap::new();
                fn find(d: &mut [usize], mut x: usize) -> usize {
                    while d[x] != x {
                        d[x] = d[d[x]];
                        x = d[x];
                    }
                    x
                }
                let mut groups: Vec<(usize, Vec<u32>)> = f1.red_members.iter().enumerate().filter(|(q, mem)| f1.red.is_int[*q] && mem.len() > 1).map(|(q, m)| (q, m.clone())).collect();
                groups.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
                // TU の判定: 既定は剥がしと 2 色塗りの十分条件、値の文字 `n` でネットワーク行列 (とその転置) の一般の判定も
                // (`tu::tu_cols_f64_net`、analysis/drcrtu_20261111.md)。どちらも列を減らしても真のままで、連結成分ごとに閉じている
                let use_net = fl.contains('n');
                let net_max: usize = env_str!("ENOMOTO_T_FOLD_NET_MAX").and_then(|v| v.parse().ok()).unwrap_or(2_000_000);
                // ネットワークの判定に渡す非零の合計の上限 (決定的な仕事量の上限。超えたら 2 色塗りの判定だけに戻す。畳めない問題で
                // 大きな成分を組ごとに確かめ直す手間が、chromaticindex512-7 で 0.4 → 71 秒になったため)。既定は元の非零の 20 倍 + 100 万
                let nnz_p: usize = p.rows.iter().map(|r| r.len()).sum();
                let net_work_mult: usize = env_str!("ENOMOTO_T_FOLD_NET_WORK").and_then(|v| v.parse().ok()).unwrap_or(20);
                let net_budget = std::cell::Cell::new(net_work_mult * nnz_p + 1_000_000);
                let net_used = std::cell::Cell::new(0usize);
                let net_over = std::cell::Cell::new(0usize);
                let tu_test = |nr: usize, cols: &[Vec<(usize, f64)>]| -> bool {
                    if super::tu::tu_cols_f64(nr, cols) {
                        return true;
                    }
                    if !use_net {
                        return false;
                    }
                    let nnz: usize = cols.iter().map(|c| c.len()).sum();
                    if net_used.get() + nnz > net_budget.get() {
                        net_over.set(net_over.get() + 1);
                        return false;
                    }
                    net_used.set(net_used.get() + nnz);
                    super::tu::tu_cols_f64_net(nr, cols, net_max)
                };
                let (mut ok_g, mut bad_g) = (0usize, 0usize);
                // 外した理由の内訳 (V_R の行が残る・S に ±1 でない係数・TU の判定に落ちた) と、TU に落ちた行列の書き出しの数
                let (mut vr_fail, mut pm1_fail, mut tu_fail, mut n_dump) = (0usize, 0usize, 0usize, 0usize);
                // 組ごとに、打ち消しのあと残る行列 S の列 (組の和の行はまだ足さない) と打ち消しの有無。V_R の行が残る組は None
                let group_s = |mem: &[u32]| -> Option<(bool, Vec<Vec<(usize, f64)>>)> {
                    // 行ごとの組の列の係数 (λ をかけて)
                    let mut by_row: std::collections::HashMap<usize, Vec<f64>> = std::collections::HashMap::new();
                    for (k, &w) in mem.iter().enumerate() {
                        let lam = f1.col_sign[w as usize];
                        for &(i, a) in &p.cols[w as usize] {
                            let e = by_row.entry(i).or_insert_with(|| vec![0.0; mem.len()]);
                            e[k] = a * lam;
                        }
                    }
                    let mut cancel = false;
                    let mut s_cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); mem.len()];
                    let mut rows_sorted: Vec<_> = by_row.into_iter().collect();
                    rows_sorted.sort_by_key(|r| r.0);
                    for (i, vals) in rows_sorted {
                        let a0 = vals[0];
                        if a0 != 0.0 && vals.iter().all(|&v| (v - a0).abs() <= 1e-12 * a0.abs()) {
                            cancel = true;
                            continue;
                        }
                        if vr[i] {
                            return None;
                        }
                        for (k, &v) in vals.iter().enumerate() {
                            if v != 0.0 {
                                // S の係数は元の係数 (λ をかける前)
                                let w = mem[k] as usize;
                                s_cols[k].push((i, v * f1.col_sign[w]));
                            }
                        }
                    }
                    Some((cancel, s_cols))
                };
                let gdata: Vec<Option<(bool, Vec<Vec<(usize, f64)>>)>> = groups.iter().map(|(_, mem)| group_s(mem)).collect();
                if env_str!("ENOMOTO_T_FOLD_PROF").is_some() { eprintln!("FOLD_PROF mip gdata {:.3}s", t0.elapsed().as_secs_f64()); }
                // 近道: V_R の条件を満たす組を全部並べた行列が TU なら全部認める (TU の十分条件は列を減らしても成り立つので、
                // 1 組ずつ足していっても全部認められる。結果は 1 組ずつの判定と同じ)
                let all_ok = {
                    let mut cols_all: Vec<Vec<(usize, f64)>> = Vec::new();
                    let mut nr = p.m;
                    for (g, (_, mem)) in gdata.iter().zip(&groups) {
                        if let Some((cancel, sc)) = g {
                            for (k, c) in sc.iter().enumerate() {
                                let mut c = c.clone();
                                if *cancel {
                                    c.push((nr, f1.col_sign[mem[k] as usize]));
                                }
                                cols_all.push(c);
                            }
                            if *cancel {
                                nr += 1;
                            }
                        }
                    }
                    tu_test(nr, &cols_all)
                };
                if env_str!("ENOMOTO_T_FOLD_PROF").is_some() { eprintln!("FOLD_PROF mip batch {:.3}s ok {all_ok}", t0.elapsed().as_secs_f64()); }
                for ((_q, mem), g) in groups.iter().zip(gdata) {
                    let (good, cancel, mut s_cols) = match g {
                        Some((c, sc)) => (true, c, sc),
                        None => (false, false, Vec::new()),
                    };
                    if good && all_ok {
                        ok_g += 1;
                        continue;
                    }
                    // 近道 (結果は同じ): S に ±1 でない係数があれば TU の十分条件は必ず外れる (`tu::tu_cols_f64` も最初にそれで偽を返す)
                    if !good {
                        vr_fail += 1;
                    }
                    let pm1 = s_cols.iter().all(|c| c.iter().all(|&(_, a)| a == 1.0 || a == -1.0));
                    if good && !pm1 {
                        pm1_fail += 1;
                    }
                    let good = good && pm1;
                    if good {
                        if cancel {
                            for (k, &w) in mem.iter().enumerate() {
                                s_cols[k].push((nrows_m, f1.col_sign[w as usize]));
                            }
                        }
                        if cancel {
                            dsu.push(nrows_m);
                        }
                        // 新しい列が触れる行の成分
                        let mut roots: Vec<usize> = Vec::new();
                        let mut seen_roots: std::collections::HashSet<usize> = std::collections::HashSet::new();
                        for c in &s_cols {
                            for &(i, _) in c {
                                let r = find(&mut dsu, i);
                                if seen_roots.insert(r) {
                                    roots.push(r);
                                }
                            }
                        }
                        // 成分の列と新しい列を集め、行の番号を詰めて確かめる
                        let mut trial: Vec<Vec<(usize, f64)>> = Vec::new();
                        for r in &roots {
                            if let Some(cs) = comp_cols.get(r) {
                                trial.extend(cs.iter().cloned());
                            }
                        }
                        trial.extend(s_cols.iter().cloned());
                        let mut remap: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
                        let local: Vec<Vec<(usize, f64)>> = trial
                            .iter()
                            .map(|c| {
                                c.iter()
                                    .map(|&(i, a)| {
                                        let k = remap.len();
                                        (*remap.entry(i).or_insert(k), a)
                                    })
                                    .collect()
                            })
                            .collect();
                        let tu_ok = tu_test(remap.len(), &local);
                        if !tu_ok {
                            tu_fail += 1;
                            tu_dump(remap.len(), &local, s_cols.len(), &mut n_dump);
                        }
                        if tu_ok {
                            // 成分をつなぐ
                            let mut merged: Vec<Vec<(usize, f64)>> = Vec::new();
                            for r in &roots {
                                if let Some(cs) = comp_cols.remove(r) {
                                    merged.extend(cs);
                                }
                            }
                            merged.extend(s_cols);
                            if let Some(&r0) = roots.first() {
                                for &r in &roots[1..] {
                                    dsu[r] = r0;
                                }
                                comp_cols.insert(r0, merged);
                            }
                            if cancel {
                                nrows_m += 1;
                            }
                            ok_g += 1;
                            continue;
                        }
                        if cancel {
                            dsu.pop();
                        }
                    }
                    bad_g += 1;
                    for &w in mem {
                        single[w as usize] = true;
                    }
                }
                if log {
                    eprintln!(
                        "MIP_FOLD: integer groups: {} accepted, {} rejected (V_R rows {}; rejected for V_R {}, non-±1 {}, TU {}; net {} work {} over {}, panics {}, cert fails {})",
                        ok_g,
                        bad_g,
                        vr.iter().filter(|&&b| b).count(),
                        vr_fail,
                        pm1_fail,
                        tu_fail,
                        use_net,
                        net_used.get(),
                        net_over.get(),
                        super::network::NET_PANICS.with(|c| c.get()),
                        super::network::NET_VERIFY_FAIL.with(|c| c.get())
                    );
                }
                if env_str!("ENOMOTO_T_FOLD_PROF").is_some() { eprintln!("FOLD_PROF mip groups {:.3}s", t0.elapsed().as_secs_f64()); }
                Fold::build(&fold_in(p, Some(&keys), Some(&single)), opts)
            }
        }
    };
    let Some(f) = f else {
        if log {
            eprintln!(
                "MIP_FOLD: no fold (n {} m {}) in {:.3}s (reason line {}, ratio {:.3})",
                p.n,
                p.m,
                t0.elapsed().as_secs_f64(),
                crate::simplex::lp_fold::LAST_FAIL.with(|c| c.get()),
                crate::simplex::lp_fold::LAST_RATIO.with(|c| c.get())
            );
        }
        return None;
    };
    // 行の組がどれも 1 行だけ (行が 1 本もまとまらない) なら、列の組は係数の同じ列 (平行な列) の集まりで、MIP の前処理が同じ列を
    // まとめる (`parallel columns merged`)。畳んでも前処理と同じ仕事を別の変数の形 (平均・和) でするだけで、neos-2987310-joes
    // (連続列 2943 本、既定 11 秒 → 53 秒) と nw04 (整数の組、12.9 → 15.6 秒) で遅くなったので畳まない
    if f.stats.row_parts == p.m && f.stats.bip_rows == 0 && !fl.contains('p') {
        if log {
            eprintln!("MIP_FOLD: no fold: only parallel columns (n {} -> {}, rows not merged); the MIP presolve merges them", p.n, f.stats.n_red);
        }
        return None;
    }
    // 列が 1 本もまとまらない (構造の列の数が同じで、双極の列もない) なら、行の組は係数の同じ行 (平行な行) の集まりで、MIP の
    // 前処理が同じ行を除く。roll3000 (行 2295 → 2091) と neos-860300 (行 850 → 595) が 58 問の種 1〜3 の全部の回で遅くなったので畳まない
    if f.stats.n_red == p.n && f.stats.bip_cols == 0 && !fl.contains('p') {
        if log {
            eprintln!("MIP_FOLD: no fold: only parallel rows (m {} -> {}, columns not merged); the MIP presolve removes them", p.m, f.stats.m_red);
        }
        return None;
    }
    let r = &f.red;
    let q = MipProblem::from_rows(r.col_lo.clone(), r.col_up.clone(), r.cost.clone(), p.offset + r.offset, p.sense_sign, r.is_int.clone(), r.rows.clone(), r.row_lo.clone(), r.row_up.clone());
    let int_groups = f.stats.int_groups > 0;
    let st = &f.stats;
    let secs = t0.elapsed().as_secs_f64();
    if log {
        eprintln!(
            "MIP_FOLD: n {} -> {}, m {} -> {} (ints {} -> {}, int groups {} with {} cols, bip cols {}, bip rows {}, neg cols {}, absorbed {}) in {:.3}s",
            st.n,
            st.n_red,
            st.m,
            st.m_red,
            p.is_int.iter().filter(|&&b| b).count(),
            q.is_int.iter().filter(|&&b| b).count(),
            st.int_groups,
            st.int_grouped_cols,
            st.bip_cols,
            st.bip_rows,
            st.neg_cols,
            st.absorbed,
            secs
        );
    }
    Some(MipFold { fold: f, q, int_groups, secs })
}

/// 診断 `ENOMOTO_T_FOLD_TUDUMP=<ディレクトリ>` (既定では使わない。analysis/drcrtu_20261111.md): 整数の組の TU の判定に落ちた行列を
/// 1 つずつファイルに書き出す (問題ごとに先頭の 40 個、非零 50 万以下)。1 行目は `行の数 列の数 新しい組の列の数` (新しい組の列は後ろ)、
/// 2 行目からは列ごとに `行:係数` を空白で並べたもの。ファイル名は `<ENOMOTO_T_FOLD_TUDUMP_TAG>_<番号>.txt`。
fn tu_dump(nrows: usize, cols: &[Vec<(usize, f64)>], new_cols: usize, n_dump: &mut usize) {
    let Some(dir) = env_str!("ENOMOTO_T_FOLD_TUDUMP") else { return };
    let nnz: usize = cols.iter().map(|c| c.len()).sum();
    if *n_dump >= 40 || nnz > 500_000 {
        return;
    }
    let tag = env_str!("ENOMOTO_T_FOLD_TUDUMP_TAG").unwrap_or("p");
    let mut s = format!("{nrows} {} {new_cols}\n", cols.len());
    for c in cols {
        let v: Vec<String> = c.iter().map(|&(i, a)| format!("{i}:{a}")).collect();
        s.push_str(&v.join(" "));
        s.push('\n');
    }
    let _ = std::fs::write(format!("{dir}/{tag}_{:03}.txt", *n_dump), s);
    *n_dump += 1;
}

impl MipFold {
    /// 畳んだ問題の解 y を元の問題の解に戻す (整数の組があれば繊維の LP を解く)。戻せなければ `None`。
    pub fn lift(&self, p: &MipProblem, y: &[f64]) -> Option<Vec<f64>> {
        let f = &self.fold;
        let mut x = f.unfold(y);
        if self.int_groups {
            // 繊維: 1 列だけの整数列は値に固定、整数の組は和 Σ λ_w x_w = y_q + Σ λ_w δ_w を固定、連続列は自由
            let mut lo = p.col_lo.clone();
            let mut up = p.col_up.clone();
            for j in 0..p.n {
                let qq = f.col_red[j];
                let grouped = qq != u32::MAX && f.red_scale[qq as usize] > 1.0;
                if p.is_int[j] && !grouped {
                    let v = x[j].round();
                    lo[j] = v;
                    up[j] = v;
                }
            }
            let mut rows = p.rows.clone();
            let mut rlo = p.row_lo.clone();
            let mut rup = p.row_up.clone();
            for (qq, mem) in f.red_members.iter().enumerate() {
                if f.red_scale[qq] <= 1.0 {
                    continue;
                }
                let mut row: Vec<(usize, f64)> = mem.iter().map(|&w| (w as usize, f.col_sign[w as usize])).collect();
                row.sort_by_key(|e| e.0);
                let s = y[qq].round() + mem.iter().map(|&w| f.col_sign[w as usize] * f.col_off[w as usize]).sum::<f64>();
                rows.push(row);
                rlo.push(s);
                rup.push(s);
            }
            let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&lo, &up, &p.cost, &rows, &rlo, &rup);
            let st = lp.solve(&super::lp::SolveLimits::default());
            if st != super::lp::LpStatus::Optimal {
                eprintln!("MIP_FOLD: the fiber LP ended with {st:?}");
                return None;
            }
            x = lp.col_values();
        }
        for j in 0..p.n {
            if p.is_int[j] {
                let v = x[j].round();
                if (x[j] - v).abs() > 1e-6 {
                    eprintln!("MIP_FOLD: lifted value of integer column {j} is not integral ({})", x[j]);
                    return None;
                }
                x[j] = v;
            }
        }
        p.is_feasible(&x, 1e-6).then_some(x)
    }
}

/// `ENOMOTO_MIP_FOLD`: 畳んで `solve` で解き、元に戻す。畳まないなら `None`。安全網に外れたら残りの時間で元を `solve` で解く。
pub fn solve_folded(p: &MipProblem, time_limit: f64, verbose: bool, solve: &dyn Fn(&MipProblem, f64) -> MipResult) -> Option<MipResult> {
    if flags().is_some_and(|f| f.contains('c')) {
        // 数えるだけ (畳んだ数を出して解かずに終える)
        let _ = try_fold(p, true);
        return Some(MipResult { status: MipStatus::NotSolved, x: None, objective: None, best_bound: f64::NEG_INFINITY, nodes: 0, lp_iterations: 0 });
    }
    let mf = try_fold(p, verbose)?;
    let t0 = std::time::Instant::now();
    let mut r = solve(&mf.q, (time_limit - mf.secs).max(0.0));
    let left = || (time_limit - mf.secs - t0.elapsed().as_secs_f64()).max(0.0);
    let fallback = |why: &str| {
        eprintln!("MIP_FOLD: safety net: {why}; solving the original ({:.2}s left)", left());
        solve(p, left())
    };
    match r.status {
        MipStatus::Infeasible => return Some(r),
        MipStatus::Unbounded | MipStatus::InfeasibleOrUnbounded if !mf.int_groups => return Some(r),
        MipStatus::Unbounded | MipStatus::InfeasibleOrUnbounded | MipStatus::NotSolved if r.x.is_none() => return Some(fallback("the folded problem gave no solution")),
        _ => {}
    }
    if let Some(y) = r.x.take() {
        let zr = r.objective.unwrap_or(mf.q.objective(&y));
        match mf.lift(p, &y) {
            Some(x) => {
                let z = p.objective(&x);
                if verbose || !flags().is_some_and(|f| f.contains('q')) {
                    eprintln!("MIP_FOLD: lifted solution objective {} (folded {}), status {:?}", p.sense_sign * z, p.sense_sign * zr, r.status);
                }
                if z > zr + 1e-6 * (1.0 + zr.abs()) {
                    // 整数の組で戻した解が畳んだ問題の値より悪い: 最適の証明にならない
                    if r.status == MipStatus::Optimal {
                        return Some(fallback("the lifted solution is worse than the folded optimum"));
                    }
                }
                r.objective = Some(z);
                r.x = Some(x);
            }
            None => return Some(fallback("the lifted solution is not feasible")),
        }
    }
    Some(r)
}

/// 試作 (既定では使わない): 値の文字 `P` (前処理 → 畳む。畳んだ問題は前処理をかけずに解く) と `Q` (前処理 → 畳む → 前処理) で、
/// MIP の前処理をかけた問題を畳む (論文は PaPILO の後に畳む)。どちらもなければ今まで通り元の問題を畳む ([`solve_folded`]、畳む → 前処理)。
/// 戻すのは 3 段 (畳んだ問題の後処理 → 配る (整数の組は繊維の LP) → 前の前処理の後処理) で、最後に元の問題での実行可能性を確かめる。
/// `solve` は前処理をかける求解、`solve_np` はかけない求解。
pub fn solve_folded_any(
    p: &MipProblem,
    time_limit: f64,
    verbose: bool,
    solve: &dyn Fn(&MipProblem, f64) -> MipResult,
    solve_np: &dyn Fn(&MipProblem, f64) -> MipResult,
) -> Option<MipResult> {
    let fl = flags()?;
    let pre_p = fl.contains('P');
    if !pre_p && !fl.contains('Q') {
        return solve_folded(p, time_limit, verbose, solve);
    }
    let t0 = std::time::Instant::now();
    let (prob, unpre) = super::presolve_for_fold(p, false)?;
    let secs_pre = t0.elapsed().as_secs_f64();
    if verbose || !fl.contains('q') {
        eprintln!("MIP_FOLD: presolved before folding: n {} -> {}, m {} -> {} in {:.3}s", p.n, prob.n, p.m, prob.m, secs_pre);
    }
    let inner: &dyn Fn(&MipProblem, f64) -> MipResult = if pre_p { solve_np } else { solve };
    let mut r = solve_folded(&prob, (time_limit - secs_pre).max(0.0), verbose, inner)?;
    if let Some(xr) = r.x.take() {
        let mut x = unpre(&xr);
        for j in 0..p.n {
            if p.is_int[j] {
                x[j] = x[j].round();
            }
        }
        if p.is_feasible(&x, 1e-6) {
            let z = p.objective(&x);
            r.best_bound += z - prob.objective(&xr);
            r.objective = Some(z);
            r.x = Some(x);
        } else {
            let left = (time_limit - t0.elapsed().as_secs_f64()).max(0.0);
            eprintln!("MIP_FOLD: safety net: the solution mapped back through the presolve is not feasible; solving the original ({left:.2}s left)");
            return Some(solve(p, left));
        }
    }
    Some(r)
}

/// 試作 `ENOMOTO_MIP_FOLD_COUNT` (診断、既定では使わない。analysis/drcr2_20261111.md の (3)): 分枝・固定の後のノード、サブ MIP、再スタートの後の
/// 問題で、畳める構造がどれだけ出るかを数えて 1 行出す (畳みはしない)。固定された列は右辺に移してから、規則 (平行な列・行だけなら畳まない)
/// を外した `rip` の手順で畳んだときの大きさを数える。`tag` は場所 (`node d=10`・`submip`・`restart 1` など)。
pub fn count_hook(p: &MipProblem, lo: &[f64], up: &[f64], tag: &str) {
    if env_str!("ENOMOTO_MIP_FOLD_COUNT").is_none() {
        return;
    }
    let t0 = std::time::Instant::now();
    // 固定された列を除き、行の範囲をずらす
    let keep: Vec<bool> = (0..p.n).map(|j| lo[j] < up[j]).collect();
    let mut newid = vec![usize::MAX; p.n];
    let mut k = 0;
    for j in 0..p.n {
        if keep[j] {
            newid[j] = k;
            k += 1;
        }
    }
    let mut rows = Vec::with_capacity(p.m);
    let (mut rlo, mut rup) = (Vec::with_capacity(p.m), Vec::with_capacity(p.m));
    for i in 0..p.m {
        let mut sh = 0.0;
        let mut r = Vec::new();
        for &(j, a) in &p.rows[i] {
            if keep[j] {
                r.push((newid[j], a));
            } else {
                sh += a * lo[j];
            }
        }
        if r.is_empty() {
            continue;
        }
        rows.push(r);
        rlo.push(p.row_lo[i] - sh);
        rup.push(p.row_up[i] - sh);
    }
    let pick = |v: &[f64]| -> Vec<f64> { (0..p.n).filter(|&j| keep[j]).map(|j| v[j]).collect() };
    let q = MipProblem::from_rows(pick(lo), pick(up), pick(&p.cost), 0.0, p.sense_sign, (0..p.n).filter(|&j| keep[j]).map(|j| p.is_int[j]).collect(), rows, rlo, rup);
    let r = try_fold_with(&q, false, "ripq");
    match r {
        Some(f) => {
            let st = &f.fold.stats;
            eprintln!(
                "MIP_FOLD_COUNT {tag}: free n {} m {} (of n {}) -> n {} m {} (int groups {} with {} cols, bip cols {}, bip rows {}) in {:.3}s",
                q.n, q.m, p.n, st.n_red, st.m_red, st.int_groups, st.int_grouped_cols, st.bip_cols, st.bip_rows, t0.elapsed().as_secs_f64()
            );
        }
        None => eprintln!("MIP_FOLD_COUNT {tag}: free n {} m {} (of n {}) -> no fold in {:.3}s", q.n, q.m, p.n, t0.elapsed().as_secs_f64()),
    }
}

thread_local! {
    /// [`count_hook`] で数えたノードの深さ (同じ深さは 1 回だけ)
    static COUNTED_DEPTHS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// ノードで数える (`ENOMOTO_MIP_FOLD_COUNT`、深さ 0・5・10・20・30・50 の最初のノードで 1 回ずつ)。
pub fn count_node(p: &MipProblem, lo: &[f64], up: &[f64], depth: usize) {
    if env_str!("ENOMOTO_MIP_FOLD_COUNT").is_none() || ![0usize, 5, 10, 20, 30, 50].contains(&depth) {
        return;
    }
    if COUNTED_DEPTHS.with(|c| {
        let mut c = c.borrow_mut();
        if c.contains(&depth) {
            true
        } else {
            c.push(depth);
            false
        }
    }) {
        return;
    }
    count_hook(p, lo, up, &format!("node d={depth}"));
}
