//! 試作 `ENOMOTO_MIP_GEN_LAZY` (既定では使わない。analysis/genlazy_20261117/): 木に入るとき、根の LP 解で余裕のある不等式の元の行を
//! LP から外し (境界を両側とも無限にする。行の番号は変えない)、ノードの LP 解が破ればカットの行として戻す (遅延制約)。
//! `ENOMOTO_MIP_CLQ_LAZY` (treeclose 9.80 節、クリーク型の行だけ) を一般の行に広げたもの。
//!
//! - 外す行: 片側だけ有限の境界の行 (両側の行・等式は外さない) で、LP のスラックが基底にあり、正規化した活動 `a` が θ
//!   (`ENOMOTO_T_MIP_GEN_LAZY_THETA`、既定 0.5) 以下のもの。`<=` の向きに直した行 `sum c_j x_j <= b` (`>=` は −1 倍) について、
//!   大域の範囲での最小活動を `L` として `a = (c·x − L) / (b − L)` (0 = 最小活動、1 = 張り付き)。`L` が無限 (範囲の無限な列を含む) の行は
//!   余裕 `s = b − c·x` を係数の 2 ノルムで割った距離 `d = s / |c|` で `a = 1 / (1 + d)` とする (距離 1 で 0.5)。
//! - 行の種類 (固定されていない列で見る): `clq` クリーク型 (2 値列の ±1 で文字の和 <= 1)、`one` 1 列、`vb` 2 列 (変数上下界など)、
//!   `knap` 2 値列だけ、`int` 整数列だけ (一般の整数を含む)、`mix` 整数と連続の混ざり、`cont` 連続だけ。
//!   `ENOMOTO_MIP_GEN_LAZY` の値に種類をコンマで並べるとその種類だけ外す (`1` か `all` は全部)。
//! - 外した行は問題 (`self.p`) に残るので、伝播・暫定解の確かめ (`try_incumbent`) は元の行で行う。LP の空の行の双対・Farkas・tableau の
//!   重み・被約費用は [`crate::simplex::mip_lp`] が 0 として扱う (CLQ_LAZY と同じ)。
//! - 戻し: ノードの LP を解くたびに (整数の判定の前に) 外した行を全部確かめ (行を詰めて持つ疎な形で、手間は外した行の非零の数)、
//!   `c·x > b + tol·max(1, |b|)` (`ENOMOTO_T_MIP_GEN_LAZY_VIOL`、既定 1e-7、FEASTOL より小さい) の行をカットの行として加えて解き直す。
//!   カットの行は年齢で消されたりノードの切り替えで消えたりしうるので、**戻した行も一覧に残して毎回確かめる** (消えていれば
//!   また戻す)。同じノードの解き直しの中で同じ行を 2 度は戻さない (LP の誤差で破り続ける行で回り続けないように)。
//! - 強分岐の子 (`ENOMOTO_MIP_GEN_LAZY_SB=1`、既定では使わない): 子の LP は外した行を確かめないので子の下界が弱くなりうる。この切り替えで、
//!   打ち切られなかった子の LP 解が破る外した行を覚え、分枝の選びの後に戻して (カットの行として加え) 分枝せずにノードの LP を解き直す
//!   (子の下界・擬費用の観測はその前の弱い LP のまま)。
//! - 診断 `ENOMOTO_MIP_DIAG_GENLAZY` (または詳しい表示): 種類ごとの片側の行の数・スラックが基底の数・非零の数と、正規化した活動の分布を
//!   `GENLAZY {...}` の 1 行 (JSON) で出す。値 `stop` なら数えるだけで外さない。

use super::Solver;
use crate::mip::lp::VarStatus;
use crate::mip::lp_api::MipLp;
use std::time::Instant;

/// 行の種類の名前 (診断と選択の文字列で使う)
pub(super) const KINDS: [&str; 7] = ["clq", "one", "vb", "knap", "int", "mix", "cont"];

/// 正規化した活動の分布の区切り (この値未満、最後は張り付き)
const BINS: [f64; 7] = [0.1, 0.25, 0.5, 0.75, 0.9, 0.99, 1.0 - 1e-9];

/// 試作 `ENOMOTO_MIP_GEN_LAZY` が使われているか
pub(super) fn on() -> bool {
    env_str!("ENOMOTO_MIP_GEN_LAZY").is_some_and(|v| v != "0")
}

/// 強分岐の子の LP 解でも外した行を確かめるか (`ENOMOTO_MIP_GEN_LAZY_SB`)
fn sb_on() -> bool {
    env_str!("ENOMOTO_MIP_GEN_LAZY_SB").is_some_and(|v| v != "0")
}

/// 外す行の種類 (`ENOMOTO_MIP_GEN_LAZY` の値。`1`・`all`・空なら全部)
fn kind_mask() -> [bool; 7] {
    let v = env_str!("ENOMOTO_MIP_GEN_LAZY").unwrap_or("1");
    if v == "1" || v == "all" || v.is_empty() {
        return [true; 7];
    }
    let mut m = [false; 7];
    for t in v.split(',') {
        if let Some(k) = KINDS.iter().position(|&s| s == t.trim()) {
            m[k] = true;
        }
    }
    m
}

/// 外した行 (`<=` の向きに直したもの) を詰めて持つ。`start[k]..start[k+1]` が行 `k` の非零
#[derive(Clone, Default)]
pub(crate) struct GenLazy {
    /// 元の行の番号
    pub(super) rows: Vec<u32>,
    start: Vec<u32>,
    col: Vec<u32>,
    val: Vec<f64>,
    rhs: Vec<f64>,
    /// 行の種類 ([`KINDS`] の番号)
    kind: Vec<u8>,
    /// 最後に戻したときの `node_lps` (同じノードの解き直しで 2 度戻さない)
    stamp: Vec<u64>,
    /// 戻した回数
    restores: Vec<u32>,
    /// 強分岐の子の LP 解が破った行 (まだ戻していない)
    pending: Vec<u32>,
}

impl GenLazy {
    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    fn push(&mut self, i: usize, r: &[(usize, f64)], sg: f64, b: f64, kind: u8) {
        if self.start.is_empty() {
            self.start.push(0);
        }
        self.rows.push(i as u32);
        for &(j, a) in r {
            self.col.push(j as u32);
            self.val.push(sg * a);
        }
        self.start.push(self.col.len() as u32);
        self.rhs.push(b);
        self.kind.push(kind);
        self.stamp.push(u64::MAX);
        self.restores.push(0);
    }
    pub(super) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
    pub(super) fn clear_pending(&mut self) {
        self.pending.clear();
    }
    /// 行 `k` の活動 (`<=` の向き) が右辺を許容誤差より超えるか
    fn violated(&self, k: usize, x: &[f64], tol: f64) -> bool {
        let (s, e) = (self.start[k] as usize, self.start[k + 1] as usize);
        let mut act = 0.0;
        for t in s..e {
            act += self.val[t] * x[self.col[t] as usize];
        }
        let b = self.rhs[k];
        act > b + tol * b.abs().max(1.0)
    }
    fn cut_row(&self, k: usize) -> (Vec<(usize, f64)>, f64, f64) {
        let (s, e) = (self.start[k] as usize, self.start[k + 1] as usize);
        let mut r: Vec<(usize, f64)> = (s..e).map(|t| (self.col[t] as usize, self.val[t])).collect();
        r.sort_by_key(|e| e.0);
        (r, f64::NEG_INFINITY, self.rhs[k])
    }
    pub(super) fn nnz(&self) -> usize {
        self.col.len()
    }
    /// `keep[k]` の行だけ残す (従が自分の基底で外せない行を除く)
    fn filter(&self, keep: impl Fn(usize) -> bool) -> GenLazy {
        let mut g = GenLazy::default();
        for k in 0..self.len() {
            if keep(self.rows[k] as usize) {
                let (s, e) = (self.start[k] as usize, self.start[k + 1] as usize);
                let r: Vec<(usize, f64)> = (s..e).map(|t| (self.col[t] as usize, self.val[t])).collect();
                g.push(self.rows[k] as usize, &r, 1.0, self.rhs[k], self.kind[k]);
            }
        }
        g
    }
}

impl<L: MipLp> Solver<'_, L> {
    /// 試作 `ENOMOTO_MIP_GEN_LAZY` の外し (木に入るとき。クリークの CLQ_LAZY の後)。主 (写しなし) は今の LP 解で行を選び、
    /// 従は写しの一覧のうち自分の基底でスラックが基底にある行を使う。
    pub(super) fn gen_lazy_drop(&mut self) {
        let t0 = Instant::now();
        let m0 = self.p.rows.len();
        if self.lp.num_rows() < m0 {
            return;
        }
        let basis = self.lp.basis();
        let slack_basic = |i: usize| basis.row.is_empty() || basis.row.get(i).is_none_or(|s| *s == VarStatus::Basic);
        let diag = env_str!("ENOMOTO_MIP_DIAG_GENLAZY");
        let cand: GenLazy = if let Some(sn) = self.snap.as_ref() {
            sn.gen_lazy.filter(|i| i < m0 && slack_basic(i) && { let (l, u) = self.lp.row_bounds(i); !(l == f64::NEG_INFINITY && u == f64::INFINITY) })
        } else {
            let p = self.p;
            let n = p.n;
            let x = self.lp.col_values();
            let theta = tunable!("ENOMOTO_T_MIP_GEN_LAZY_THETA", 0.5, f64);
            let mask = kind_mask();
            let glo = &self.dom.global_lo;
            let gup = &self.dom.global_up;
            let binary: Vec<bool> = (0..n).map(|j| p.is_int[j] && glo[j] >= 0.0 && gup[j] <= 1.0).collect();
            let free: Vec<bool> = (0..n).map(|j| glo[j] < gup[j]).collect();
            // 診断: 種類ごとの [片側の行, スラックが基底, その非零, 範囲が無限, 外した行, 外した非零] と、活動の分布 (スラックが基底の行)
            let mut cnt = [[0usize; 6]; 7];
            let mut hist = [[0usize; 8]; 7];
            let nnz_all: usize = p.rows.iter().map(|r| r.len()).sum();
            let mut n_two = 0usize;
            let mut out = GenLazy::default();
            for (i, r) in p.rows.iter().enumerate() {
                let (l, u) = self.lp.row_bounds(i);
                if l == f64::NEG_INFINITY && u == f64::INFINITY {
                    continue;
                }
                if l.is_finite() && u.is_finite() {
                    n_two += 1;
                    continue;
                }
                let (sg, b) = if u.is_finite() { (1.0, u) } else { (-1.0, -l) };
                // 種類 (固定されていない列で見る)
                let rr: Vec<(usize, f64)> = r.iter().copied().filter(|e| free[e.0]).collect();
                if rr.is_empty() {
                    continue;
                }
                let shift: f64 = r.iter().filter(|e| !free[e.0]).map(|&(j, a)| sg * a * glo[j]).sum();
                let nint = rr.iter().filter(|e| p.is_int[e.0]).count();
                let kind: u8 = if rr.len() >= 2 && rr.iter().all(|e| binary[e.0]) && {
                    let c = rr[0].1.abs();
                    let nneg = rr.iter().filter(|e| sg * e.1 < 0.0).count() as f64;
                    rr.iter().all(|e| (e.1.abs() - c).abs() <= 1e-12 * c.max(1.0)) && (((b - shift) / c + nneg) - 1.0).abs() <= 1e-9
                } {
                    0
                } else if rr.len() == 1 {
                    1
                } else if rr.len() == 2 {
                    2
                } else if rr.iter().all(|e| binary[e.0]) {
                    3
                } else if nint == rr.len() {
                    4
                } else if nint > 0 {
                    5
                } else {
                    6
                };
                let kk = kind as usize;
                cnt[kk][0] += 1;
                if !slack_basic(i) {
                    continue;
                }
                cnt[kk][1] += 1;
                cnt[kk][2] += r.len();
                // 正規化した活動
                let act: f64 = r.iter().map(|&(j, a)| sg * a * x[j]).sum();
                let mut lmin = 0.0f64;
                for &(j, a) in r {
                    let c = sg * a;
                    lmin += if c > 0.0 { c * glo[j] } else { c * gup[j] };
                }
                let s = b - act;
                let na = if lmin.is_finite() {
                    if b - lmin <= 1e-9 * b.abs().max(1.0) {
                        1.0
                    } else {
                        ((act - lmin) / (b - lmin)).clamp(0.0, 1.0)
                    }
                } else {
                    cnt[kk][3] += 1;
                    let nrm: f64 = r.iter().map(|e| e.1 * e.1).sum::<f64>().sqrt().max(1e-300);
                    1.0 / (1.0 + s.max(0.0) / nrm)
                };
                let bin = BINS.iter().position(|&t| na < t).unwrap_or(7);
                hist[kk][bin] += 1;
                if mask[kk] && na <= theta && s > 1e-9 * b.abs().max(1.0) {
                    cnt[kk][4] += 1;
                    cnt[kk][5] += r.len();
                    out.push(i, r, sg, b, kind);
                }
            }
            if diag.is_some() || (self.params.verbose && self.pool_tid == 0) {
                let ks: Vec<String> = (0..7).map(|k| format!("\"{}\":{{\"rows\":{},\"basic\":{},\"nnz\":{},\"infrange\":{},\"drop\":{},\"dropnnz\":{},\"hist\":{:?}}}", KINDS[k], cnt[k][0], cnt[k][1], cnt[k][2], cnt[k][3], cnt[k][4], cnt[k][5], hist[k])).collect();
                eprintln!("GENLAZY {{\"m\":{m0},\"nnz\":{nnz_all},\"two_sided\":{n_two},\"theta\":{theta},\"bins\":{BINS:?},{}}}", ks.join(","));
            }
            out
        };
        if diag == Some("stop") || cand.is_empty() {
            return;
        }
        let ch: Vec<(usize, f64, f64)> = cand.rows.iter().map(|&i| (i as usize, f64::NEG_INFINITY, f64::INFINITY)).collect();
        self.lp.set_row_bounds_many(&ch);
        self.gen_lazy_stats[0] += cand.len() as u64;
        self.gen_lazy_stats[5] += cand.nnz() as u64;
        if self.params.verbose && self.pool_tid == 0 {
            let nnz_all: usize = self.p.rows.iter().map(|r| r.len()).sum();
            eprintln!("MIP: GEN_LAZY: {} rows dropped from the LP (of {} rows; {} of {} nonzeros) ({:.2}s)", cand.len(), m0, cand.nnz(), nnz_all, t0.elapsed().as_secs_f64());
        }
        self.gen_lazy = cand;
    }

    /// 試作 `ENOMOTO_MIP_GEN_LAZY` の戻し: 今の LP 解が破る外した行をカットの行として LP に加える (一覧には残す)。戻したら真。
    pub(super) fn gen_lazy_restore(&mut self) -> bool {
        let t0 = Instant::now();
        self.gen_lazy_stats[1] += 1;
        let x = self.lp.col_values();
        let tol = tunable!("ENOMOTO_T_MIP_GEN_LAZY_VIOL", 1e-7, f64);
        let stamp = self.node_lps;
        let g = &mut self.gen_lazy;
        g.pending.clear();
        let mut back: Vec<(Vec<(usize, f64)>, f64, f64)> = Vec::new();
        for k in 0..g.rows.len() {
            if g.stamp[k] != stamp && g.violated(k, &x, tol) {
                g.stamp[k] = stamp;
                if g.restores[k] > 0 {
                    self.gen_lazy_stats[4] += 1;
                }
                g.restores[k] += 1;
                back.push(g.cut_row(k));
            }
        }
        self.gen_lazy_secs += t0.elapsed().as_secs_f64();
        if back.is_empty() {
            return false;
        }
        self.gen_lazy_stats[2] += back.len() as u64;
        self.add_cut_rows(&back);
        true
    }

    /// 強分岐の子の LP 解 `x` が破る外した行を覚える (`ENOMOTO_MIP_GEN_LAZY_SB`)
    pub(super) fn gen_lazy_sb_note(&mut self, x: &[f64]) {
        if !sb_on() {
            return;
        }
        let t0 = Instant::now();
        let tol = tunable!("ENOMOTO_T_MIP_GEN_LAZY_VIOL", 1e-7, f64);
        let stamp = self.node_lps;
        let g = &mut self.gen_lazy;
        for k in 0..g.rows.len() {
            if g.stamp[k] != stamp && g.violated(k, x, tol) {
                // 同じノードで 2 度覚えない (戻すときに stamp を付け直す)
                g.stamp[k] = stamp;
                g.pending.push(k as u32);
            }
        }
        self.gen_lazy_secs += t0.elapsed().as_secs_f64();
    }

    /// 強分岐の子で覚えた行をカットの行として戻す。戻したら真 (呼び手はノードの LP を解き直す)
    pub(super) fn gen_lazy_sb_flush(&mut self) -> bool {
        let g = &mut self.gen_lazy;
        if g.pending.is_empty() {
            return false;
        }
        let pend = std::mem::take(&mut g.pending);
        let mut back = Vec::with_capacity(pend.len());
        for &k in &pend {
            let k = k as usize;
            if g.restores[k] > 0 {
                self.gen_lazy_stats[4] += 1;
            }
            g.restores[k] += 1;
            back.push(g.cut_row(k));
        }
        self.gen_lazy_stats[2] += back.len() as u64;
        self.gen_lazy_stats[6] += back.len() as u64;
        self.gen_lazy_stats[7] += 1;
        self.add_cut_rows(&back);
        true
    }

    /// 終わりの表示 (詳しい表示のとき)
    pub(super) fn gen_lazy_report(&self) {
        let s = self.gen_lazy_stats;
        let g = &self.gen_lazy;
        let mut by_kind = [0u64; 7];
        let mut rows_back = 0usize;
        for k in 0..g.len() {
            if g.restores[k] > 0 {
                rows_back += 1;
                by_kind[g.kind[k] as usize] += 1;
            }
        }
        eprintln!(
            "MIP: GEN_LAZY stats tid {}: dropped {} (nnz {}) checks {} restored {} (distinct rows {}, by kind {:?}) re-restored {} resolves {} sb restored {} sb resolves {} check secs {:.3}",
            self.pool_tid, s[0], s[5], s[1], s[2], rows_back, by_kind, s[4], s[3], s[6], s[7], self.gen_lazy_secs
        );
    }
}
