//! `ENOMOTO_MIP_LMIP_SWAP` (試作、既定では使わない。BINHEUR、analysis/binheur_20261111.md): 集合分割の行
//! (等式・0-1 列だけ・係数がすべて同じで「ちょうど 1 本」) の中で、1 を立てる列を入れ替える組の手を Local-MIP の候補に足す。
//!
//! 1 列ずつの反転では、集合分割の行の 1 を別の列へ移すのに「0 → 1 で行を破る (2 本になる)」か「1 → 0 で行を破る (0 本になる)」を
//! 一度通らなければならない。ここでは違反行 (目的の行を含む) の列 j の tight move が 0-1 の反転で、j が集合分割の行 q にあるとき:
//!
//! - j を 0 → 1 にするなら、q で今 1 の列 k を 1 → 0 にする組 (q を保ったまま 1 を j へ移す)
//! - j を 1 → 0 にするなら、q で今 0 の列 k (乱数で `ENOMOTO_T_MIP_LMIP_SWAP_PARTNERS` 本まで) を 0 → 1 にする組
//!
//! をスコア ([`St::score2`]) で比べる。暫定解の後は目的の行が違反行になるので、目的の列の tight move (費用のある列を下げる) に
//! 相手を付けた組が、目的を良くしながら集合分割の行を保つ入れ替えになる。

use super::St;

/// 集合分割の行の索引と、行ごとの「今 1 の列」の記録
pub(super) struct SwapIdx {
    /// 集合分割の行 (問題の行の番号)
    rows: Vec<usize>,
    /// 列 → その列を含む集合分割の行 (`rows` の位置)
    col_rows: Vec<Vec<u32>>,
    /// 行ごとに最後に見つけた 1 の列 (古ければ行を見直す)
    one: Vec<usize>,
    /// 計測: 評価した組の数 (`ENOMOTO_MIP_LOCALMIP_DEBUG` で出す)
    pub n_eval: u64,
}

/// 組の手の候補: (スコア, breakthrough, j, j の新しい値, k, k の新しい値)
pub(super) type SwapCand = (f64, bool, usize, f64, usize, f64);

impl SwapIdx {
    /// 集合分割の行を集める。なければ None
    pub(super) fn build(s: &St) -> Option<Self> {
        let p = s.p;
        let mut rows = Vec::new();
        let mut col_rows = vec![Vec::new(); p.n];
        for i in 0..p.m {
            let r = &p.rows[i];
            if p.row_lo[i] != p.row_up[i] || r.len() < 2 {
                continue;
            }
            let a = r[0].1;
            if a == 0.0 || (p.row_lo[i] / a - 1.0).abs() > 1e-12 {
                continue;
            }
            // 0 に固定された列は入ってよい (候補にしない)。1 に固定された列があれば行は動かせないので除く
            if !r.iter().all(|&(j, b)| b == a && p.is_int[j] && s.lo[j] == 0.0 && (s.up[j] == 1.0 || s.up[j] == 0.0)) {
                continue;
            }
            let q = rows.len() as u32;
            rows.push(i);
            for &(j, _) in r {
                col_rows[j].push(q);
            }
        }
        if rows.is_empty() {
            return None;
        }
        Some(SwapIdx { one: vec![usize::MAX; rows.len()], rows, col_rows, n_eval: 0 })
    }

    /// 行 q で今 1 の列 (`except` を除く)。なければ None
    fn one_in(&mut self, s: &St, q: usize, except: usize) -> Option<usize> {
        let c = self.one[q];
        if c != usize::MAX && c != except && s.x[c] > 0.5 {
            return Some(c);
        }
        for &(k, _) in &s.p.rows[self.rows[q]] {
            if k != except && s.x[k] > 0.5 {
                self.one[q] = k;
                return Some(k);
            }
        }
        None
    }

    /// 違反行 `i` の列を `cols` 本まで見て、集合分割の行の中の入れ替えの組の最良を返す
    pub(super) fn best_swap(&mut self, s: &mut St, i: usize, cols: usize, partners: usize, step: u64, buf: &mut Vec<(usize, f64)>) -> Option<SwapCand> {
        let mut best: Option<SwapCand> = None;
        let len = s.row(i).len();
        for t in 0..len.min(cols) {
            let q = if len <= cols { t } else { s.below(len) };
            let (j, a) = s.row(i)[q];
            if self.col_rows[j].is_empty() {
                continue;
            }
            let Some(v) = s.tight_value(i, j, a) else { continue };
            let d = v - s.x[j];
            if d.abs() != 1.0 || !s.tabu_ok(j, d, step) {
                continue;
            }
            // 列の集合分割の行は最初の 2 本まで (1 本目で行を保てば、2 本目以降は score2 が破れを数える)
            for qi in 0..self.col_rows[j].len().min(2) {
                let qr = self.col_rows[j][qi] as usize;
                if d > 0.0 {
                    let Some(k) = self.one_in(s, qr, j) else { continue };
                    self.n_eval += u64::from(eval_pair(s, j, d, v, k, -1.0, step, buf, &mut best));
                } else {
                    let row = self.rows[qr];
                    let rl = s.p.rows[row].len();
                    for _ in 0..partners {
                        let k = s.p.rows[row][s.below(rl)].0;
                        if k != j && s.x[k] < 0.5 && s.up[k] == 1.0 {
                            self.n_eval += u64::from(eval_pair(s, j, d, v, k, 1.0, step, buf, &mut best));
                        }
                    }
                }
            }
        }
        best
    }
}

/// 組 (j を d、k を e 動かす) を評価して `best` より良ければ置き換える。評価したら真
#[allow(clippy::too_many_arguments)]
fn eval_pair(s: &St, j: usize, d: f64, v: f64, k: usize, e: f64, step: u64, buf: &mut Vec<(usize, f64)>, best: &mut Option<SwapCand>) -> bool {
    if !s.tabu_ok(k, e, step) {
        return false;
    }
    let m = s.p.m;
    let sc = s.score2(j, d, k, e, buf);
    let bt = s.obj_up.is_finite() && s.act[m] + s.p.cost[j] * d + s.p.cost[k] * e <= s.obj_up + super::TOL;
    if best.as_ref().is_none_or(|b| sc > b.0 || (sc == b.0 && bt && !b.1)) {
        *best = Some((sc, bt, j, v, k, s.x[k] + e));
    }
    true
}
