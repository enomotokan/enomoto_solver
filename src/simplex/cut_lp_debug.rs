//! 根のカットのループで書き出した LP (`ENOMOTO_MIP_DEBUG_DUMP_CUT_LP`、[`super::mip_lp::TwoStageLp::dump_lp`]) を、
//! 同じ出発の基底から MIP 用の LP ラッパーで解き直す診断テスト (`#[ignore]`、analysis/lp_hang_20261021.md)。
//! `CUTLP=<prefix> cargo test --release --lib cut_lp_debug -- --ignored --nocapture`
//! (`<prefix>.mps` と `<prefix>.bas` を読む。`CUTLP_COLD=1` で基底を使わず全スラック基底から、`CUTLP_TIME=<秒>` で
//! 時間の上限、`CUTLP_ITERS=<回>` で反復の上限、`CUTLP_REPEAT=<回>` で同じ求解を繰り返す)。

use super::mip_lp::TwoStageLp;
use crate::mip::lp::SolveLimits;
use std::collections::HashMap;

/// [`crate::mip::problem::MipProblem::write_mps`] が書く自由 MPS を読む (最小化形)。
/// 返り値: (lo, up, cost, rows, row_lo, row_up)。
pub(crate) fn read_mps(path: &str) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<f64>) {
    let s = std::fs::read_to_string(path).unwrap();
    let mut sec = "";
    let mut row_id: HashMap<String, usize> = HashMap::new();
    let mut row_kind: Vec<char> = Vec::new();
    let mut col_id: HashMap<String, usize> = HashMap::new();
    let (mut lo, mut up, mut cost): (Vec<f64>, Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new(), Vec::new());
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut rhs: Vec<f64> = Vec::new();
    let mut rng: Vec<Option<f64>> = Vec::new();
    let mut obj_name = String::new();
    let p = |t: &str| -> f64 { t.parse::<f64>().unwrap() };
    for line in s.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        if t.is_empty() {
            continue;
        }
        if !line.starts_with(' ') {
            sec = t[0];
            continue;
        }
        match sec {
            "ROWS" => {
                let k = t[0].chars().next().unwrap();
                if k == 'N' {
                    obj_name = t[1].to_string();
                } else {
                    row_id.insert(t[1].to_string(), rows.len());
                    row_kind.push(k);
                    rows.push(Vec::new());
                    rhs.push(0.0);
                    rng.push(None);
                }
            }
            "COLUMNS" => {
                if t.len() >= 3 && t[1] == "'MARKER'" {
                    continue;
                }
                let j = *col_id.entry(t[0].to_string()).or_insert_with(|| {
                    lo.push(0.0);
                    up.push(f64::INFINITY);
                    cost.push(0.0);
                    lo.len() - 1
                });
                let mut k = 1;
                while k + 1 < t.len() {
                    let v = p(t[k + 1]);
                    if t[k] == obj_name {
                        cost[j] = v;
                    } else if v != 0.0 {
                        rows[row_id[t[k]]].push((j, v));
                    }
                    k += 2;
                }
            }
            "RHS" => {
                let mut k = 1;
                while k + 1 < t.len() {
                    if t[k] != obj_name {
                        rhs[row_id[t[k]]] = p(t[k + 1]);
                    }
                    k += 2;
                }
            }
            "RANGES" => {
                let mut k = 1;
                while k + 1 < t.len() {
                    rng[row_id[t[k]]] = Some(p(t[k + 1]));
                    k += 2;
                }
            }
            "BOUNDS" => {
                let j = col_id[t[2]];
                match t[0] {
                    "FX" => {
                        lo[j] = p(t[3]);
                        up[j] = p(t[3]);
                    }
                    "LO" => lo[j] = p(t[3]),
                    "UP" => up[j] = p(t[3]),
                    "MI" => lo[j] = f64::NEG_INFINITY,
                    "PL" => up[j] = f64::INFINITY,
                    "FR" => {
                        lo[j] = f64::NEG_INFINITY;
                        up[j] = f64::INFINITY;
                    }
                    "BV" => {
                        lo[j] = 0.0;
                        up[j] = 1.0;
                    }
                    _ => panic!("bound {}", t[0]),
                }
            }
            _ => {}
        }
    }
    let m = rows.len();
    let (mut row_lo, mut row_up) = (vec![f64::NEG_INFINITY; m], vec![f64::INFINITY; m]);
    for i in 0..m {
        match row_kind[i] {
            'E' => {
                row_lo[i] = rhs[i];
                row_up[i] = rhs[i];
            }
            'G' => {
                row_lo[i] = rhs[i];
                if let Some(r) = rng[i] {
                    row_up[i] = rhs[i] + r.abs();
                }
            }
            _ => {
                row_up[i] = rhs[i];
                if let Some(r) = rng[i] {
                    row_lo[i] = rhs[i] - r.abs();
                }
            }
        }
    }
    (lo, up, cost, rows, row_lo, row_up)
}

/// `.bas` を読む: (基底列の番号, 直近の解 x, 引き継ぐ DSE 重み (基底位置順。なければ空))。
pub(crate) fn read_bas(path: &str) -> (Vec<usize>, Vec<f64>, Vec<f64>) {
    let s = std::fs::read_to_string(path).unwrap();
    let mut lines = s.lines();
    let _ = lines.next();
    let b: Vec<usize> = lines.next().unwrap_or("").split_whitespace().map(|t| t.parse().unwrap()).collect();
    let x: Vec<f64> = lines.next().unwrap_or("").split_whitespace().map(|t| t.parse().unwrap()).collect();
    let w: Vec<f64> = lines.next().unwrap_or("").split_whitespace().map(|t| t.parse().unwrap()).collect();
    (b, x, w)
}

#[test]
#[ignore]
fn cut_lp_debug() {
    let prefix = std::env::var("CUTLP").unwrap();
    let (lo, up, cost, rows, row_lo, row_up) = read_mps(&format!("{prefix}.mps"));
    let (b, x, w) = read_bas(&format!("{prefix}.bas"));
    let n = lo.len();
    let m = rows.len();
    let nnz: usize = rows.iter().map(|r| r.len()).sum();
    eprintln!("CUTLP: {prefix}: n={n} m={m} nnz={nnz} basis={} x={} dse={}", b.len(), x.len(), w.len());
    // `CUTLP_DSE=1`: 4 行目の DSE 重みを渡して引き継ぐ (`ENOMOTO_WARM_DSE=1` も要る)。既定は渡さない (単位重みから)
    let use_dse = std::env::var("CUTLP_DSE").is_ok() && w.len() == m;
    // `CUTLP_DSE_NEWEXACT=1`: 重み 1 の行 (加えたばかりの行) を未知 (`NaN`) にして、求解の始めに厳密な値を埋めさせる
    // (`ENOMOTO_MIP_ROWS_DSE_EXACT` の動作を書き出した LP で試す)
    let w: Vec<f64> = if std::env::var("CUTLP_DSE_NEWEXACT").is_ok() { w.iter().map(|&v| if v == 1.0 { f64::NAN } else { v }).collect() } else { w };
    // `CUTLP_DSE_JITTER=<相対量>`: 重みに相対 `±相対量` の決定的な揺らぎ (`CUTLP_SEED` で変える) を入れる (経路の敏感さを見る)
    let jitter: f64 = std::env::var("CUTLP_DSE_JITTER").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let seed: u64 = std::env::var("CUTLP_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let w: Vec<f64> = if jitter > 0.0 {
        w.iter().enumerate().map(|(i, &v)| {
            let mut h = (i as u64).wrapping_add(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            h ^= h >> 31;
            let r = (h >> 11) as f64 / (1u64 << 53) as f64;
            v * (1.0 + jitter * (2.0 * r - 1.0))
        }).collect()
    } else {
        w
    };
    let cold = std::env::var("CUTLP_COLD").is_ok();
    let time: Option<f64> = std::env::var("CUTLP_TIME").ok().and_then(|v| v.parse().ok());
    let iters: u64 = std::env::var("CUTLP_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(u64::MAX);
    let repeat: usize = std::env::var("CUTLP_REPEAT").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    for rep in 0..repeat {
        let mut lp = TwoStageLp::new(&lo, &up, &cost, &rows, &row_lo, &row_up);
        if !cold && b.len() == m {
            lp.set_basis_cols(&b, if x.len() == n { Some(&x) } else { None });
            // `CUTLP_DSE_COMPARE=1`: 4 行目の重み (引き継いだ重み) と、この基底の厳密な重みを比べる
            if std::env::var("CUTLP_DSE_COMPARE").is_ok() && w.len() == m {
                compare_weights(&lp, &b, &w, "start");
            }
            if use_dse {
                let pairs: Vec<(u32, f32)> = b.iter().zip(&w).map(|(&v, &x)| (v as u32, x as f32)).collect();
                lp.set_dse_vars(&pairs);
                let n1 = w.iter().filter(|&&v| v == 1.0).count();
                let nn = w.iter().filter(|v| v.is_nan()).count();
                eprintln!("CUTLP: DSE weights passed: {} rows, {} with weight 1, {} unknown, max {:.3e}", w.len(), n1, nn, w.iter().cloned().filter(|v| !v.is_nan()).fold(0.0, f64::max));
            }
        }
        let deadline = time.map(|t| std::time::Instant::now() + std::time::Duration::from_secs_f64(t));
        let t0 = std::time::Instant::now();
        let st = lp.solve(&SolveLimits { iteration_limit: iters, cutoff: f64::INFINITY, deadline });
        let secs = t0.elapsed().as_secs_f64();
        let it = lp.total_iterations();
        eprintln!("CUTLP: rep={rep} status={st:?} iters={it} time={secs:.3}s ({:.1} us/iter) obj={:.10e}", 1e6 * secs / it.max(1) as f64, lp.objective());
        // 求解の後の基底について、次の求解に渡る重み (求解の最後の重み) と厳密な重みを比べる
        if std::env::var("CUTLP_DSE_COMPARE").is_ok() {
            if let (Some(b2), Some(w2)) = (lp.basis_cols(), lp.dse_weights()) {
                compare_weights(&lp, &b2, &w2, "end");
            }
        }
    }
}

/// 重み `w` (基底 `b` の位置順) と `lp` の今の基底の厳密な重みを比べて表示する。
fn compare_weights(lp: &TwoStageLp, b: &[usize], w: &[f64], when: &str) {
    let m = w.len();
    {
        {
                if let Some(ex) = lp.exact_dse_weights() {
                    let mut ratios: Vec<(f64, usize)> = (0..m).filter(|&i| !w[i].is_nan()).map(|i| (w[i] / ex[i], i)).collect();
                    ratios.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                    let k = ratios.len();
                    let cnt = |lo: f64, hi: f64| ratios.iter().filter(|r| r.0 >= lo && r.0 < hi).count();
                    eprintln!(
                        "CUTLP: {when}: inherited/exact weight ratio over {k} rows: min {:.3e} p1 {:.3e} median {:.3e} p99 {:.3e} max {:.3e}; <0.01: {} <0.1: {} [0.1,10): {} >=10: {} >=100: {} >=1e4: {}; exact max {:.3e} p99 {:.3e} median {:.3e}",
                        ratios[0].0, ratios[k / 100].0, ratios[k / 2].0, ratios[k * 99 / 100].0, ratios[k - 1].0,
                        cnt(0.0, 0.01), cnt(0.0, 0.1), cnt(0.1, 10.0), cnt(10.0, f64::INFINITY), cnt(100.0, f64::INFINITY), cnt(1e4, f64::INFINITY),
                        ex.iter().cloned().fold(0.0, f64::max), { let mut e = ex.clone(); e.sort_by(|a, b| a.partial_cmp(b).unwrap()); e[m * 99 / 100] }, { let mut e = ex.clone(); e.sort_by(|a, b| a.partial_cmp(b).unwrap()); e[m / 2] }
                    );
                    // 比が最も大きい 5 行: 基底変数 (構造列 j < n、スラック n + i)
                    let top: Vec<String> = ratios.iter().rev().take(5).map(|&(r, i)| format!("pos {i} var {} w {:.3e} exact {:.3e} ratio {:.3e}", b[i], w[i], ex[i], r)).collect();
                    eprintln!("CUTLP: {when}: largest ratios: {}", top.join(" | "));
                }
        }
    }
}
