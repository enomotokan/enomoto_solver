//! 試作 `ENOMOTO_MIP_HNFCUT` (既定では使わない): 試作 `ENOMOTO_MIP_HNFPRE` ([`super::hnf`]) の列ごとの合同式
//! `x_j ≡ r_j (mod g_j)` を、問題を置き換えずにカットの分離器に教える。整数点では `x_j = r_j + g_j z_j` (`z_j` は整数) なので、
//! 分離器の中だけで列を `z_j` に写し (係数 `a_j g_j`、右辺から `a_j r_j` を引く)、丸めてから `x` に戻す。LP 緩和の値は変わらない
//! (アフィン変換) が、丸めが「`x_j` は `g_j` 刻み」を使えるので、合同式に合わない整数の LP 点も切れる。
//!
//! 文字で使う場所を選ぶ (`1` = `abc`):
//! - `a`: CMIR (集約行・経路集約・元の行 1 本ずつの [`super::cuts::generate_cuts`]) の項の置き換え ([`super::cuts`] の `Sub::Cong`)。
//!   置き換えた空間と元の空間の両方で CMIR を作り、効き目の大きい方を残す。
//! - `b`: zerohalf と `ENOMOTO_MIP_SNFMODK` の CG カット (元の行から作るもの) を、列を `z` に写した行・上下限・LP 値で作って戻す ([`map_rows`])。
//! - `c`: 単体表の行 (tableau 行) の CMIR (GMI に当たる) に `a` と同じ置き換えを使う。
//! - `l` (`1` には含めない): 行の活動量の合同式。整数の行 `s = Σ a_j x_j` で合同式の列を含むものは、`s ≡ Σ a_j r_j (mod G)`
//!   (`G` = 合同式の列の `a_j g_j` とほかの整数列の `a_j` の gcd) なので、`a`・`c` では論理変数 (行の活動量) も `s = r + G t` と置き換え
//!   ([`extend_logicals`])、`b` では写した行の上下限を `G` の倍数に丸める ([`map_rows`])。
//!
//! `b` は `ENOMOTO_MIP_MODKVB` の連立 (`modkvb_cuts`) にも、変数・論理変数・変数上下限を写して使う ([`map_cutvars`])。
//! 診断: `ENOMOTO_MIP_HNFCUT_BPAR=even` / `odd` なら `b` で法が偶数 / 奇数の列だけ写す。
//!
//! `ENOMOTO_MIP_HNFCUT` だけで `ENOMOTO_MIP_HNFPRE` がなければ、`HNFPRE=1` と同じ前処理・丸めをする ([`super::hnf::mode`])。
//! 表は [`super::domain::Domain::cong`] (問題の指紋で照らしたもの) を分離の間だけスレッドごとに置く ([`Guard`])。

use super::cuts::RawCut;
use super::hnf::CongTable;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// `ENOMOTO_MIP_HNFCUT` の文字 (`a`・`b`・`c`)。空なら使わない。
pub(crate) fn letters() -> &'static str {
    static L: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    L.get_or_init(|| match env_str!("ENOMOTO_MIP_HNFCUT") {
        None | Some("") | Some("0") => String::new(),
        Some("1") => "abc".to_string(),
        Some(s) => s.to_string(),
    })
}

pub(crate) fn on() -> bool {
    !letters().is_empty()
}

pub(crate) fn has(c: char) -> bool {
    letters().contains(c)
}

/// 分離の間のスレッドごとの状態: 表と、今の分離で CMIR・mod-k に使うか。
#[derive(Clone, Default)]
pub(crate) struct State {
    pub table: Option<Arc<CongTable>>,
    pub cmir: bool,
    pub modk: bool,
    /// tableau 行の区間にいるか (数え上げ用)
    pub tab: bool,
    /// `l` (行の活動量の合同式) を使うか
    pub logi: bool,
}

thread_local! {
    static CUR: RefCell<State> = RefCell::new(State::default());
}

/// 今のスレッドの状態の写し (並列の分離のスレッドに渡す)。
pub(crate) fn snapshot() -> State {
    CUR.with(|c| c.borrow().clone())
}

/// 状態を置き、落とすときに元に戻す。
pub(crate) struct Guard(Option<State>);

impl Guard {
    pub fn new(s: State) -> Guard {
        Guard(Some(CUR.with(|c| c.replace(s))))
    }
    /// 使わないときは何もしない (状態を触らない)。
    pub fn none() -> Guard {
        Guard(None)
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(s) = self.0.take() {
            CUR.with(|c| *c.borrow_mut() = s);
        }
    }
}

/// CMIR に使う表 (今の分離で CMIR に使うときだけ)。
pub(crate) fn with_cmir_table<R>(f: impl FnOnce(Option<&CongTable>) -> R) -> R {
    CUR.with(|c| {
        let s = c.borrow();
        if s.cmir {
            f(s.table.as_deref())
        } else {
            f(None)
        }
    })
}

/// 分離の最初に置く状態 (`kind` = `'a'` (行・集約) か `'c'` (tableau 行))。
pub(crate) fn state_for(table: Option<Arc<CongTable>>, kind: char) -> State {
    if table.is_none() || !on() {
        return State::default();
    }
    State { table, cmir: has(kind), modk: has('b'), tab: kind == 'c', logi: has('l') }
}

/// 今の状態の CMIR の可否だけを `kind` (`'a'` か `'c'`) に合わせて変える (tableau 行の区間の出入り)。表がなければ何もしない
pub(crate) fn set_kind(kind: char) {
    CUR.with(|c| {
        let mut s = c.borrow_mut();
        if s.table.is_some() {
            s.cmir = has(kind);
            s.tab = kind == 'c';
        }
    });
}

/// tableau 行の区間か
pub(crate) fn in_tab() -> bool {
    CUR.with(|c| c.borrow().tab)
}

/// 数え上げ: 0 = 合同式の項を含む CMIR の行, 1 = 置き換えた方のカットが勝った, 2 = 元の方だけ作れた, 3 = 置き換えた方だけ作れた,
/// 4 = mod-k の呼び出し (写した), 5 = 写した列の数の合計, 6 = mod-k のカット, 7 = (未使用), 8 = tableau 行で 0 の数,
/// 9 = 論理変数の合同式 (分離の回ごとの合計), 10 = `b` で丸めた行, 11 = 写した mod-k のカットのうち合同式の列を含むもの,
/// 12 = MODKVB を写して呼んだ数, 13 = その写したカット
pub(crate) static STATS: [AtomicU64; 16] = [const { AtomicU64::new(0) }; 16];
/// 診断: 分離の LP 点での合同式の列の値 (最初の分離 = 根の最初の LP の点): 0 = 列の数, 1 = x が分数, 2 = x は整数だが合同式に合わない,
/// 3 = 合同式に合う, 4 = 上下限に乗っている (3 のうち), 5 = 数えたか
pub(crate) static LPPT: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];

/// 根の最初の分離の LP 点で合同式の列の値を数える (1 回だけ)
pub(crate) fn note_lp_point(t: &CongTable, x: &[f64], lo: &[f64], up: &[f64]) {
    if LPPT[5].swap(1, Ordering::Relaxed) == 1 {
        return;
    }
    for k in 0..t.n.min(x.len()) {
        if t.m[k] <= 1 || lo[k] >= up[k] {
            continue;
        }
        LPPT[0].fetch_add(1, Ordering::Relaxed);
        let v = x[k];
        let i = if (v - v.round()).abs() > 1e-6 {
            1
        } else if (v.round() as i64 - t.r[k]).rem_euclid(t.m[k]) != 0 {
            2
        } else {
            if (v - lo[k]).abs() < 1e-6 || (v - up[k]).abs() < 1e-6 {
                LPPT[4].fetch_add(1, Ordering::Relaxed);
            }
            3
        };
        LPPT[i].fetch_add(1, Ordering::Relaxed);
    }
    if env_str!("ENOMOTO_MIP_LOG").is_some() && env_str!("ENOMOTO_MIP_HNFPRE_QUIET").is_none() {
        let q: Vec<u64> = LPPT.iter().map(|a| a.load(Ordering::Relaxed)).collect();
        eprintln!("HNFCUT: first LP point: cong cols {} (x frac {}, int but off {}, on {} (at bound {}))", q[0], q[1], q[2], q[3], q[4]);
    }
}

/// 置き換えた方が勝ったときの効き目の合計 (×1e6) と、元の方の効き目の合計 (×1e6、両方作れたとき)。
pub(crate) static EFF: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];

pub(crate) fn stat(i: usize) {
    STATS[i].fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn stats_line() -> String {
    let s: Vec<u64> = STATS.iter().map(|a| a.load(Ordering::Relaxed)).collect();
    let e: Vec<f64> = EFF.iter().map(|a| a.load(Ordering::Relaxed) as f64 * 1e-6).collect();
    let q: Vec<u64> = LPPT.iter().map(|a| a.load(Ordering::Relaxed)).collect();
    format!(
        "HNFCUT stats: first LP point: cong cols {} (x frac {}, int but off {}, on {} (at bound {})); cmir rows with cong {} (tab {}), cong won {} (eff {:.3} vs orig {:.3}), only orig {}, only cong {}; modk calls {} mapped cols {} cuts {} (with cong cols {}), rows rounded {}; logical congs {}; modkvb mapped calls {} cuts {}",
        q[0], q[1], q[2], q[3], q[4], s[0], s[8], s[1], e[0], e[1], s[2], s[3], s[4], s[5], s[6], s[11], s[10], s[9], s[12], s[13]
    )
}

/// 列 `k` の `lo..=up` に入る `z = (x - r)/g` の範囲 `(lz, uz)` (無限の側は無限)。空なら `None`。
pub(crate) fn z_range(t: &CongTable, k: usize, lo: f64, up: f64) -> Option<(f64, f64)> {
    let (r, g) = (t.r[k] as f64, t.m[k] as f64);
    let lz = if lo.is_finite() { ((lo - r) / g - 1e-9).ceil() } else { f64::NEG_INFINITY };
    let uz = if up.is_finite() { ((up - r) / g + 1e-9).floor() } else { f64::INFINITY };
    if lz > uz || (lo.is_finite() && lo.abs() > 1e12) || (up.is_finite() && up.abs() > 1e12) {
        return None;
    }
    Some((lz, uz))
}

/// 合同式の列か (表の範囲内・法 > 1・整数列・固定されていない)。
#[inline]
pub(crate) fn is_cong(t: &CongTable, k: usize, is_int: &[bool], lo: &[f64], up: &[f64]) -> bool {
    k < t.n && k < is_int.len() && t.m[k] > 1 && is_int[k] && lo[k] < up[k]
}

/// 元の行から作る分離器 (zerohalf・`ENOMOTO_MIP_SNFMODK` の CG) を、合同式の列を `z` に写した空間で呼ぶ (`b`)。
/// `f(rows, row_lo, row_up, is_int, lo, up, x)` のカット (構造変数 `0..n` の係数) を `x` の空間に戻して返す。
/// 使わない (表がない・`b` でない・写す列がない) ときは元のまま `f` を呼ぶ。
#[allow(clippy::too_many_arguments)]
pub(crate) fn map_rows(
    rows: &[Vec<(usize, f64)>],
    row_lo: &[f64],
    row_up: &[f64],
    is_int: &[bool],
    lo: &[f64],
    up: &[f64],
    x: &[f64],
    f: impl FnOnce(&[Vec<(usize, f64)>], &[f64], &[f64], &[bool], &[f64], &[f64], &[f64]) -> Vec<RawCut>,
) -> Vec<RawCut> {
    let (st, logi) = CUR.with(|c| {
        let s = c.borrow();
        (if s.modk { s.table.clone() } else { None }, s.logi)
    });
    let Some(t) = st else {
        return f(rows, row_lo, row_up, is_int, lo, up, x);
    };
    let n = lo.len().min(t.n);
    // 列 → (r, g)。写せない列は g = 0
    let mut rg: Vec<(f64, f64)> = vec![(0.0, 0.0); n];
    let (mut zlo, mut zup, mut zx) = (lo.to_vec(), up.to_vec(), x.to_vec());
    let mut ncols = 0u64;
    let bpar = env_str!("ENOMOTO_MIP_HNFCUT_BPAR");
    for k in 0..n {
        if !is_cong(&t, k, is_int, lo, up) {
            continue;
        }
        match bpar {
            Some("even") if t.m[k] % 2 != 0 => continue,
            Some("odd") if t.m[k] % 2 == 0 => continue,
            _ => {}
        }
        let Some((lz, uz)) = z_range(&t, k, lo[k], up[k]) else { continue };
        let (r, g) = (t.r[k] as f64, t.m[k] as f64);
        rg[k] = (r, g);
        zlo[k] = lz;
        zup[k] = uz;
        zx[k] = (x[k] - r) / g;
        ncols += 1;
    }
    if ncols == 0 {
        return f(rows, row_lo, row_up, is_int, lo, up, x);
    }
    stat(4);
    STATS[5].fetch_add(ncols, Ordering::Relaxed);
    // 行: a x = a (r + g z) なので、係数 a g、上下限から a r を引く
    let mut zrows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(rows.len());
    let (mut zrlo, mut zrup) = (row_lo.to_vec(), row_up.to_vec());
    for (i, row) in rows.iter().enumerate() {
        let mut shift = 0.0;
        let mut zr = Vec::with_capacity(row.len());
        for &(j, a) in row {
            if j < n && rg[j].1 > 0.0 {
                zr.push((j, a * rg[j].1));
                shift += a * rg[j].0;
            } else {
                zr.push((j, a));
            }
        }
        if shift != 0.0 {
            zrlo[i] -= shift;
            zrup[i] -= shift;
        }
        // `l`: 合同式の列を含む整数の行は、写した係数の gcd G の倍数に上下限を丸める (固定された列は定数として扱う)
        if logi && row.iter().any(|&(j, _)| j < n && rg[j].1 > 0.0) {
            if let Some((g, c)) = row_gcd(&zr, is_int, &zlo, &zup) {
                if g > 1 {
                    let gf = g as f64;
                    let (l2, u2) = (zrlo[i] - c, zrup[i] - c);
                    let l3 = if l2.is_finite() { gf * (l2 / gf - 1e-9).ceil() } else { l2 };
                    let u3 = if u2.is_finite() { gf * (u2 / gf + 1e-9).floor() } else { u2 };
                    if l3 <= u3 && (l3 > l2 + 1e-9 || u3 < u2 - 1e-9) {
                        zrlo[i] = l3 + c;
                        zrup[i] = u3 + c;
                        stat(10);
                    }
                }
            }
        }
        zrows.push(zr);
    }
    let cuts = f(&zrows, &zrlo, &zrup, is_int, &zlo, &zup, &zx);
    // 戻す: c z = c (x - r) / g
    let mut out = Vec::with_capacity(cuts.len());
    for mut c in cuts {
        let mut ok = true;
        for e in c.coefs.iter_mut() {
            if e.0 < n && rg[e.0].1 > 0.0 {
                let (r, g) = rg[e.0];
                e.1 /= g;
                c.rhs += e.1 * r;
            } else if e.0 >= lo.len() {
                ok = false;
            }
        }
        if ok && c.rhs.is_finite() {
            if c.coefs.iter().any(|e| e.0 < n && rg[e.0].1 > 0.0 && e.1 != 0.0) {
                stat(11);
            }
            out.push(c);
        }
    }
    STATS[6].fetch_add(out.len() as u64, Ordering::Relaxed);
    out
}

/// 整数 (誤差 1e-9 以内、絶対値 1e9 以下) なら i64
fn as_int(a: f64) -> Option<i64> {
    let r = a.round();
    ((a - r).abs() <= 1e-9 * (1.0 + a.abs()) && r.abs() <= 1e9).then_some(r as i64)
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// 行 `row` (列は整数で係数は整数) の固定されていない列の係数の gcd と、固定された列の定数。整数でない列・係数があれば `None`
fn row_gcd(row: &[(usize, f64)], is_int: &[bool], lo: &[f64], up: &[f64]) -> Option<(i64, f64)> {
    let mut g = 0i64;
    let mut c = 0.0;
    for &(j, a) in row {
        if a == 0.0 {
            continue;
        }
        if lo[j] == up[j] {
            c += a * lo[j];
            continue;
        }
        if !is_int[j] {
            return None;
        }
        g = gcd(g, as_int(a)?);
    }
    (g > 0 && c.is_finite()).then_some((g, c))
}

/// `l`: 分離の回ごとに、LP の行 (論理変数 `n + i`) の合同式を足した表を作って今の状態に置く。整数の行 `s = Σ a_j x_j` で
/// 合同式の列を含むものは `s ≡ Σ_{合同式} a_j r_j + Σ_{固定} a_j x_j (mod G)`、`G = gcd(合同式の列の a_j g_j, ほかの整数列の a_j)`。
/// `is_int`・`lo`・`up` は論理変数を含む (長さ `n + 行の数`)。表がない・`l` でないときは何もしない
pub(crate) fn extend_logicals(lp_rows: &[Vec<(usize, f64)>], is_int: &[bool], lo: &[f64], up: &[f64], n: usize) {
    let Some(t) = CUR.with(|c| {
        let s = c.borrow();
        if s.logi {
            s.table.clone()
        } else {
            None
        }
    }) else {
        return;
    };
    if t.n != n || is_int.len() < n + lp_rows.len() {
        return;
    }
    let mut r = t.r.clone();
    let mut m = t.m.clone();
    r.resize(n + lp_rows.len(), 0);
    m.resize(n + lp_rows.len(), 0);
    let mut cnt = 0u64;
    'rows: for (i, row) in lp_rows.iter().enumerate() {
        let k = n + i;
        if !is_int[k] || lo[k] >= up[k] {
            continue;
        }
        let mut g = 0i64;
        let mut res = 0i128;
        let mut any = false;
        for &(j, a) in row {
            if a == 0.0 {
                continue;
            }
            let Some(ai) = as_int(a) else { continue 'rows };
            if lo[j] == up[j] {
                let Some(v) = as_int(lo[j]) else { continue 'rows };
                res += ai as i128 * v as i128;
            } else if j < n && t.m[j] > 1 {
                let Some(ag) = ai.checked_mul(t.m[j]) else { continue 'rows };
                g = gcd(g, ag);
                res += ai as i128 * t.r[j] as i128;
                any = true;
            } else if is_int[j] {
                g = gcd(g, ai);
            } else {
                continue 'rows;
            }
            if res.abs() > 1i128 << 100 {
                continue 'rows;
            }
        }
        if !any || g <= 1 {
            continue;
        }
        r[k] = res.rem_euclid(g as i128) as i64;
        m[k] = g;
        cnt += 1;
    }
    STATS[9].fetch_add(cnt, Ordering::Relaxed);
    if cnt > 0 {
        let ext = Arc::new(CongTable { fp: t.fp, n: n + lp_rows.len(), r, m });
        CUR.with(|c| c.borrow_mut().table = Some(ext));
    }
}

/// `b` を `ENOMOTO_MIP_MODKVB` の連立に使うための写し: 構造変数の合同式の列を `z` に、論理変数を `s - shift` にした変数の情報・LP の行・
/// 変数上下限。[`MappedVars::unmap`] でカットを元の空間に戻す。
pub(crate) struct MappedVars {
    pub lo: Vec<f64>,
    pub up: Vec<f64>,
    pub x: Vec<f64>,
    pub is_int: Vec<bool>,
    pub rows: Vec<Vec<(usize, f64)>>,
    pub vb: super::cuts::VarBounds,
    rg: Vec<(f64, f64)>,
    shift: Vec<f64>,
    n: usize,
}

/// [`MappedVars`] を作る (今の状態で `b` が使えて、写す列があるときだけ)。`lo`・`up`・`x`・`is_int` は構造変数 `0..n` と論理変数 `n..n+m`
pub(crate) fn map_cutvars(vars: &super::cuts::CutVars, lp_rows: &[Vec<(usize, f64)>], n: usize) -> Option<MappedVars> {
    let t = CUR.with(|c| {
        let s = c.borrow();
        if s.modk {
            s.table.clone()
        } else {
            None
        }
    })?;
    let m = lp_rows.len();
    if vars.lo.len() < n + m || t.n < n {
        return None;
    }
    let mut rg = vec![(0.0, 0.0); n];
    let (mut lo, mut up, mut x) = (vars.lo.to_vec(), vars.up.to_vec(), vars.x.to_vec());
    let mut any = false;
    for k in 0..n {
        if !is_cong(&t, k, vars.is_int, vars.lo, vars.up) {
            continue;
        }
        let Some((lz, uz)) = z_range(&t, k, vars.lo[k], vars.up[k]) else { continue };
        let (r, g) = (t.r[k] as f64, t.m[k] as f64);
        rg[k] = (r, g);
        lo[k] = lz;
        up[k] = uz;
        x[k] = (vars.x[k] - r) / g;
        any = true;
    }
    if !any {
        return None;
    }
    let mut rows = Vec::with_capacity(m);
    let mut shift = vec![0.0; m];
    for (i, row) in lp_rows.iter().enumerate() {
        let mut zr = Vec::with_capacity(row.len());
        for &(j, a) in row {
            if j < n && rg[j].1 > 0.0 {
                zr.push((j, a * rg[j].1));
                shift[i] += a * rg[j].0;
            } else {
                zr.push((j, a));
            }
        }
        let k = n + i;
        lo[k] -= shift[i];
        up[k] -= shift[i];
        x[k] -= shift[i];
        rows.push(zr);
    }
    // 変数上下限: 相手 y が合同式の列なら d x_y + e = d g z + (e + d r)。自分が合同式の列の整数の変数上下限は捨てる
    let mut vb = super::cuts::VarBounds::default();
    if let Some(v) = vars.vb {
        let mp = |l: &Vec<Vec<(usize, f64, f64)>>| -> Vec<Vec<(usize, f64, f64)>> {
            l.iter().map(|e| e.iter().map(|&(y, d, ee)| if y < n && rg[y].1 > 0.0 { (y, d * rg[y].1, ee + d * rg[y].0) } else { (y, d, ee) }).collect()).collect()
        };
        vb.vub = mp(&v.vub);
        vb.vlb = mp(&v.vlb);
        let drop = |l: &Vec<Vec<(usize, f64, f64)>>| -> Vec<Vec<(usize, f64, f64)>> {
            l.iter().enumerate().map(|(k, e)| if k < n && rg[k].1 > 0.0 { Vec::new() } else { e.iter().filter(|&&(z, _, _)| !(z < n && rg[z].1 > 0.0)).copied().collect() }).collect()
        };
        vb.ivub = drop(&v.ivub);
        vb.ivlb = drop(&v.ivlb);
    }
    stat(12);
    Some(MappedVars { lo, up, x, is_int: vars.is_int.to_vec(), rows, vb, rg, shift, n })
}

impl MappedVars {
    pub fn vars(&self) -> super::cuts::CutVars<'_> {
        super::cuts::CutVars { lo: &self.lo, up: &self.up, is_int: &self.is_int, x: &self.x, vb: Some(&self.vb) }
    }

    /// 写した空間のカットを元の空間に戻す (構造変数 `c z = (c/g) x - c r/g`、論理変数 `c s' = c s - c shift`)
    pub fn unmap(&self, mut c: RawCut) -> Option<RawCut> {
        for e in c.coefs.iter_mut() {
            if e.0 < self.n {
                if self.rg[e.0].1 > 0.0 {
                    let (r, g) = self.rg[e.0];
                    e.1 /= g;
                    c.rhs += e.1 * r;
                }
            } else {
                c.rhs += e.1 * self.shift.get(e.0 - self.n)?;
            }
        }
        stat(13);
        c.rhs.is_finite().then_some(c)
    }
}

/// 今の状態の CMIR の可否を `on` にして `f` を呼ぶ (写した空間で `generate_cuts` を呼ぶときに `a` の置き換えを重ねないため)
pub(crate) fn with_cmir<R>(on: bool, f: impl FnOnce() -> R) -> R {
    let old = CUR.with(|c| std::mem::replace(&mut c.borrow_mut().cmir, on));
    let r = f();
    CUR.with(|c| c.borrow_mut().cmir = old);
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mip::cuts::{generate_cuts, CutVars};

    struct Rng(u64);
    impl Rng {
        fn f(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        fn int(&mut self, a: i64, b: i64) -> i64 {
            a + (self.f() * (b - a + 1) as f64).floor() as i64
        }
    }

    /// 箱の整数点のうち合同式に合うものを全部並べる
    fn points(lo: &[f64], up: &[f64], t: &CongTable) -> Vec<Vec<f64>> {
        let n = lo.len();
        let mut out = Vec::new();
        let mut pt: Vec<f64> = lo.to_vec();
        loop {
            if (0..n).all(|j| t.m[j] <= 1 || (pt[j] as i64 - t.r[j]).rem_euclid(t.m[j]) == 0) {
                out.push(pt.clone());
            }
            let mut j = 0;
            while j < n {
                pt[j] += 1.0;
                if pt[j] <= up[j] {
                    break;
                }
                pt[j] = lo[j];
                j += 1;
            }
            if j == n {
                return out;
            }
        }
    }

    fn act(c: &[(usize, f64)], x: &[f64]) -> f64 {
        c.iter().map(|&(j, a)| a * x[j]).sum()
    }

    /// 乱数の行・合同式で、`a` (CMIR の置き換え) と `b` (zerohalf の写し) のカットが合同式に合う整数点を切らないことを確かめる。
    /// LP 点は分数の点と、合同式に合わない整数の点を混ぜる (置き換えたカットが元のカットより強いことも数える)
    #[test]
    fn hnfcut_random_validity() {
        let mut rng = Rng(12345);
        let (mut n_cmir, mut n_won, mut n_zh, mut n_cut_int_pt) = (0, 0, 0, 0);
        for it in 0..4000 {
            let n = 2 + (it % 4);
            let lo: Vec<f64> = (0..n).map(|_| rng.int(-3, 1) as f64).collect();
            let up: Vec<f64> = (0..n).map(|j| lo[j] + rng.int(2, 7) as f64).collect();
            let mut r = vec![0i64; n];
            let mut m = vec![0i64; n];
            for j in 0..n {
                if rng.f() < 0.6 {
                    m[j] = rng.int(2, 4);
                    r[j] = rng.int(-5, 5);
                }
            }
            let t = CongTable { fp: 0, n, r, m };
            let pts = points(&lo, &up, &t);
            if pts.is_empty() {
                continue;
            }
            // LP 点: 半分は分数、半分は合同式に合わない整数の点 (の近く)
            let x: Vec<f64> = (0..n)
                .map(|j| if it % 2 == 0 { lo[j] + rng.f() * (up[j] - lo[j]) } else { rng.int(lo[j] as i64, up[j] as i64) as f64 })
                .collect();
            let is_int = vec![true; n];
            let mrow = 1 + (it % 3);
            let mut rows = Vec::new();
            let (mut rl, mut ru) = (Vec::new(), Vec::new());
            for _ in 0..mrow {
                let row: Vec<(usize, f64)> = (0..n).filter_map(|j| (rng.f() < 0.8).then(|| (j, rng.int(-5, 5) as f64))).filter(|e| e.1 != 0.0).collect();
                if row.is_empty() {
                    continue;
                }
                // 右辺は LP 点の活動量の近く (切れやすいように)
                let b = (act(&row, &x) + rng.f() * 2.0 - 1.0).floor();
                rows.push(row);
                rl.push(f64::NEG_INFINITY);
                ru.push(b);
            }
            let tbl = Arc::new(t);
            let _g = Guard::new(State { table: Some(tbl.clone()), cmir: true, modk: true, tab: false, logi: false });
            let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: None };
            // a: 行 1 本ずつ
            for (i, row) in rows.iter().enumerate() {
                let feas: Vec<&Vec<f64>> = pts.iter().filter(|p| act(row, p) <= ru[i] + 1e-9).collect();
                let before = STATS[1].load(Ordering::Relaxed) + STATS[3].load(Ordering::Relaxed);
                for c in generate_cuts(&vars, row, ru[i], false, false) {
                    n_cmir += 1;
                    for p in &feas {
                        assert!(act(&c.coefs, p) <= c.rhs + 1e-6, "cmir cut {:?} <= {} cuts {:?} (row {:?} <= {}, table {:?})", c.coefs, c.rhs, p, row, ru[i], tbl);
                    }
                    if it % 2 == 1 && act(&c.coefs, &x) > c.rhs + 1e-6 {
                        n_cut_int_pt += 1;
                    }
                }
                if STATS[1].load(Ordering::Relaxed) + STATS[3].load(Ordering::Relaxed) > before {
                    n_won += 1;
                }
            }
            // b: zerohalf (行の連立)
            let feas: Vec<&Vec<f64>> = pts.iter().filter(|p| rows.iter().enumerate().all(|(i, row)| act(row, p) <= ru[i] + 1e-9)).collect();
            let cuts = map_rows(&rows, &rl, &ru, &is_int, &lo, &up, &x, |rw, a, b, ii, l, u, xx| crate::mip::zerohalf::zerohalf_cuts(rw, a, b, ii, l, u, xx, 10));
            for c in cuts {
                n_zh += 1;
                for p in &feas {
                    assert!(act(&c.coefs, p) <= c.rhs + 1e-6, "zh cut {:?} <= {} cuts {:?} (rows {:?} <= {:?}, table {:?})", c.coefs, c.rhs, p, rows, ru, tbl);
                }
            }
        }
        eprintln!("hnfcut_random_validity: cmir cuts {n_cmir} (cong won rows {n_won}, cutting an integer LP point {n_cut_int_pt}), zerohalf cuts {n_zh}");
        assert!(n_won > 100 && n_zh > 10 && n_cut_int_pt > 10);
    }

    /// `l` (論理変数の合同式・行の丸め) と MODKVB の写し ([`map_cutvars`]) の確かめ。乱数の整数の行 (論理変数つき) から tableau 行のような
    /// 集約 `Σ w_i (a_i x - s_i) = 0` を作り、(1) 表に論理変数を足した CMIR、(2) 行を丸めた zerohalf、(3) 写した変数・行で作って戻した CMIR の
    /// カットが、行の上下限と合同式を満たす全ての整数点 (論理変数はその点の活動量) を切らないこと
    #[test]
    fn hnfcut_logical_validity() {
        let mut rng = Rng(98765);
        let (mut n_l, mut n_lc, mut n_zh, mut n_mv) = (0, 0, 0, 0);
        for it in 0..3000 {
            let n = 2 + (it % 4);
            let lo: Vec<f64> = (0..n).map(|_| rng.int(-2, 1) as f64).collect();
            let up: Vec<f64> = (0..n).map(|j| lo[j] + rng.int(2, 7) as f64).collect();
            let mut r = vec![0i64; n];
            let mut m = vec![0i64; n];
            for j in 0..n {
                if rng.f() < 0.7 {
                    m[j] = rng.int(2, 4);
                    r[j] = rng.int(-5, 5);
                }
            }
            let t = CongTable { fp: 0, n, r, m };
            let pts = points(&lo, &up, &t);
            if pts.is_empty() {
                continue;
            }
            let mrow = 1 + (it % 3);
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            let x: Vec<f64> = (0..n).map(|j| if it % 2 == 0 { lo[j] + rng.f() * (up[j] - lo[j]) } else { rng.int(lo[j] as i64, up[j] as i64) as f64 }).collect();
            let (mut rl, mut ru) = (Vec::new(), Vec::new());
            for _ in 0..mrow {
                let row: Vec<(usize, f64)> = (0..n).filter_map(|j| (rng.f() < 0.8).then(|| (j, rng.int(-4, 4) as f64))).filter(|e| e.1 != 0.0).collect();
                if row.is_empty() {
                    continue;
                }
                let a = act(&row, &x);
                rl.push(if rng.f() < 0.5 { (a - rng.f() * 3.0).ceil() } else { f64::NEG_INFINITY });
                ru.push((a + rng.f() * 3.0 - 1.0).floor().max(rl.last().copied().unwrap_or(f64::NEG_INFINITY)));
                rows.push(row);
            }
            let mr = rows.len();
            if mr == 0 {
                continue;
            }
            let feas: Vec<Vec<f64>> = pts
                .iter()
                .filter(|p| rows.iter().enumerate().all(|(i, row)| act(row, p) >= rl[i] - 1e-9 && act(row, p) <= ru[i] + 1e-9))
                .map(|p| {
                    let mut q = p.clone();
                    q.extend(rows.iter().map(|row| act(row, p)));
                    q
                })
                .collect();
            // 変数 (構造 + 論理)
            let mut vlo = lo.clone();
            let mut vup = up.clone();
            let mut vx = x.clone();
            for (i, row) in rows.iter().enumerate() {
                vlo.push(rl[i]);
                vup.push(ru[i]);
                vx.push(act(row, &x));
            }
            let vint = vec![true; n + mr];
            let tbl = Arc::new(t);
            let _g = Guard::new(State { table: Some(tbl.clone()), cmir: true, modk: true, tab: false, logi: true });
            extend_logicals(&rows, &vint, &vlo, &vup, n);
            if CUR.with(|c| c.borrow().table.as_ref().is_some_and(|t| t.n > n)) {
                n_l += 1;
            }
            let check = |c: &RawCut, what: &str| {
                for q in &feas {
                    assert!(act(&c.coefs, q) <= c.rhs + 1e-6, "{what} cut {:?} <= {} cuts {:?} (rows {:?} in {:?}..{:?}, table {:?})", c.coefs, c.rhs, q, rows, rl, ru, tbl);
                }
            };
            // tableau 行のような集約 Σ w_i (a_i x - s_i) = 0 (両向き)
            let w: Vec<f64> = (0..mr).map(|_| (rng.f() - 0.5) * 3.0).collect();
            let mut base: Vec<(usize, f64)> = Vec::new();
            for (i, row) in rows.iter().enumerate() {
                for &(j, a) in row {
                    base.push((j, w[i] * a));
                }
                base.push((n + i, -w[i]));
            }
            let neg: Vec<(usize, f64)> = base.iter().map(|&(j, a)| (j, -a)).collect();
            {
                let vars = CutVars { lo: &vlo, up: &vup, is_int: &vint, x: &vx, vb: None };
                for b in [&base, &neg] {
                    for c in generate_cuts(&vars, b, 0.0, false, false) {
                        n_lc += 1;
                        check(&c, "logical cmir");
                    }
                }
                // (3) 写した変数・行 (MODKVB の形)
                if let Some(mv) = map_cutvars(&vars, &rows, n) {
                    let mut mbase: Vec<(usize, f64)> = Vec::new();
                    for (i, row) in mv.rows.iter().enumerate() {
                        for &(j, a) in row {
                            mbase.push((j, w[i] * a));
                        }
                        mbase.push((n + i, -w[i]));
                    }
                    let mneg: Vec<(usize, f64)> = mbase.iter().map(|&(j, a)| (j, -a)).collect();
                    for b in [&mbase, &mneg] {
                        for c in with_cmir(false, || generate_cuts(&mv.vars(), b, 0.0, false, false)) {
                            if let Some(c) = mv.unmap(c) {
                                n_mv += 1;
                                check(&c, "mapped vars");
                            }
                        }
                    }
                }
            }
            // (2) 行を丸めた zerohalf (構造変数だけ)
            let cuts = map_rows(&rows, &rl, &ru, &vint[..n], &lo, &up, &x, |rw, a, b, ii, l, u, xx| crate::mip::zerohalf::zerohalf_cuts(rw, a, b, ii, l, u, xx, 10));
            for c in cuts {
                n_zh += 1;
                check(&c, "zerohalf rounded");
            }
        }
        eprintln!("hnfcut_logical_validity: problems with logical congruences {n_l}, cmir cuts {n_lc}, zerohalf {n_zh}, mapped-vars cuts {n_mv}");
        assert!(n_l > 300 && n_lc > 300 && n_zh > 50 && n_mv > 300);
    }
}
