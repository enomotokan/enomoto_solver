//! 手間の単位 (決定的な時計、analysis/timedep_20261114.md の 11 節)。
//!
//! スレッドごとに、部分 (LP・伝播・分離・前処理・ヒューリスティクス・ノード) 別の手間を数える。数えは常に行う (スレッドローカルの
//! 足し算だけで、各部分の外側のループで 1 回)。重み付きの合計 [`total`] を、秒の代わりの上限 (手間の期限 [`with_deadline`]) に使う。
//! 重みは `ENOMOTO_T_TICKS_W_<部分>` (既定は時間の規則のログから当てはめた値で、合計がおよそ CPU のナノ秒になる)。
//! 期限は既定ではない (`u64::MAX`) ので、既定の道筋は変わらない。

use std::cell::Cell;

pub const LP: usize = 0;
pub const PROP: usize = 1;
pub const SEP: usize = 2;
pub const PRE: usize = 3;
pub const HEUR: usize = 4;
pub const NODE: usize = 5;
/// LP の 1 反復ごとに 1 (反復の決まった手間の分)
pub const LPIT: usize = 6;
/// LU の分解・求解・更新 (`FtLu` の tick と同じ数え。触った非零の数)
pub const LU: usize = 7;
/// LU の求解の、1 回ごとに行の数に比例する分 (配列を順に流す: 密な右辺の掃き出し・作業配列の片付けなど。非零 1 つより安い。
/// analysis/timedep_20261114.md の 12.9 節: 行の多い neos-827175 では [`LU`] と同じ重みだと CPU 時間の 2 倍に数えていた)
pub const LUM: usize = 8;
pub const NK: usize = 9;
pub const NAMES: [&str; NK] = ["lp", "prop", "sep", "pre", "heur", "node", "lpit", "lu", "lum"];

thread_local! {
    static RAW: [Cell<u64>; NK] = const { [const { Cell::new(0) }; NK] };
    static DEADLINE: Cell<f64> = const { Cell::new(f64::INFINITY) };
    /// [`vclock`]: このスレッドの時計を手間にするか
    static VCLOCK: Cell<bool> = const { Cell::new(false) };
    /// [`Gate`]: このスレッドの手間の門 (あれば [`over`] の中で許された手間まで止まって待つ)
    static GATE: std::cell::RefCell<Option<std::sync::Arc<Gate>>> = const { std::cell::RefCell::new(None) };
    static HAS_GATE: Cell<bool> = const { Cell::new(false) };
    /// [`ipm_work`]: 内点法の反復の手間を数えて期限で止めるか
    static IPM_WORK: Cell<bool> = const { Cell::new(false) };
    /// [`ipm_add`]: 内点法の反復の手間 (重み付き、[`total`] には足さない)
    static IPM_EXTRA: Cell<f64> = const { Cell::new(0.0) };
}

/// `ENOMOTO_MIP_DET1T` (既定 `onil`) の `i`: 内点法の手間 `n` (重み [`HEUR`]) を数える。スレッドの手間の合計 [`total`] には足さず、
/// [`ipm_over`] の期限の判定にだけ使う (合計に足すと、後で手間の割合で予算を決めるサブ MIP などの道が既定と変わる。rout の種 2 で 1.4 倍)
pub fn ipm_add(n: f64) {
    IPM_EXTRA.with(|e| e.set(e.get() + weights()[HEUR] * n));
}

/// [`over`] に内点法の手間 ([`ipm_add`]) も足して比べる
pub fn ipm_over() -> bool {
    over() || total() + IPM_EXTRA.with(|e| e.get()) >= DEADLINE.with(|d| d.get())
}

/// `ENOMOTO_MIP_DET1T` (既定 `onil`) の `i`: このスレッドの内点法 (`solve_box_lp`) が、反復ごとに KKT の分解の演算数を数え ([`ipm_add`])、
/// 手間の期限 ([`ipm_over`]) を過ぎたら止めるか ([`ipm_work_guard`] の間だけ真)
pub fn ipm_work() -> bool {
    IPM_WORK.with(|v| v.get())
}

/// [`ipm_work`] を真にし、捨てるときに元に戻す見張り
pub struct IpmWorkGuard(bool, f64);

pub fn ipm_work_guard() -> IpmWorkGuard {
    IpmWorkGuard(IPM_WORK.with(|v| v.replace(true)), IPM_EXTRA.with(|e| e.get()))
}

impl Drop for IpmWorkGuard {
    fn drop(&mut self) {
        IPM_WORK.with(|v| v.set(self.0));
        IPM_EXTRA.with(|e| e.set(self.1));
    }
}

fn weights() -> &'static [f64; NK] {
    static W: std::sync::OnceLock<[f64; NK]> = std::sync::OnceLock::new();
    W.get_or_init(|| {
        // 既定は 58 問の 1 スレッドの時間の規則のログ (analysis/timedep_20261114/tool/ticks_fit.py、cal4) で当てはめた重み x 1e9 (1 手間 ≒ 1 ns)
        let mut w = [3.48, 5.55, 31.7, 84.0, 16.6, 235.0, 9063.0, 2.11, 2.11];
        for (k, n) in NAMES.iter().enumerate() {
            if let Ok(v) = std::env::var(format!("ENOMOTO_T_TICKS_W_{}", n.to_uppercase())) {
                if let Ok(x) = v.parse::<f64>() {
                    w[k] = x;
                }
            }
        }
        w
    })
}

/// 部分 `k` の手間を `n` 足す。
/// 足すのは部分ごとの数だけ (重み付きの合計は読むときに計算する。数えの負荷を小さくするため)。
#[inline]
pub fn add(k: usize, n: u64) {
    RAW.with(|r| r[k].set(r[k].get().wrapping_add(n)));
}

/// 試作 `ENOMOTO_MIP_SUBMIP_TICKS` の文字 `c` (analysis/workcount_20261125、既定では使わない)。`1` は全部の文字
pub fn submip_ticks(c: char) -> bool {
    static V: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    V.get_or_init(|| std::env::var("ENOMOTO_MIP_SUBMIP_TICKS").ok().filter(|v| !v.is_empty() && v != "0")).as_deref().is_some_and(|v| v.contains(c) || v.contains('1'))
}

/// 試作 `ENOMOTO_MIP_SUBMIP_TICKS` の `d`: 診断用の手間 ([`diag_add`]、zero-half・clique・implied bound の分離器) も合計に入れる重み (入れないなら 0)
fn diag_weight() -> f64 {
    static W: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *W.get_or_init(|| if submip_ticks('d') { tunable!("ENOMOTO_T_TICKS_W_DIAG", 2.0, f64) } else { 0.0 })
}

/// このスレッドの重み付きの手間の合計。
/// 試作 `ENOMOTO_MIP_SUBMIP_TICKS` の `d` なら、診断用の手間 ([`diag_add`]) も重み `ENOMOTO_T_TICKS_W_DIAG` で足す
#[inline]
pub fn total() -> f64 {
    let w = weights();
    let wd = diag_weight();
    let base: f64 = RAW.with(|r| (0..NK).map(|k| w[k] * r[k].get() as f64).sum());
    if wd > 0.0 {
        base + wd * DIAG.with(|c| c.get()) as f64
    } else {
        base
    }
}

thread_local! {
    /// 診断用の手間 ([`diag_add`]): 合計 [`total`] にも期限にも入れない (既定の道筋を変えない)
    static DIAG: Cell<u64> = const { Cell::new(0) };
}

/// 診断用の手間を `n` 足す (分離 [`SEP`] の単位、`ENOMOTO_MIP_SEPSTAT` の分離器ごとの手間に使う。analysis/sepeff_20261120/)。
/// [`total`] に入らないので、手間の期限・時計は変わらない。今まで手間を数えていなかった分離器 (zero-half・clique など) が使う
#[inline]
pub fn diag_add(n: u64) {
    DIAG.with(|c| c.set(c.get().wrapping_add(n)));
}

/// このスレッドの [`total`] + 診断用の手間 ([`diag_add`]) (診断用)。診断用の手間の 1 単位は `ENOMOTO_T_TICKS_W_DIAG` (既定 2、
/// 単位はビット列の 1 語の xor・隣接の 1 要素ほどの軽い操作で、seymour・nw04 の根で zero-half・implied bound・clique の時間とおよそ合う)
pub fn total_with_diag() -> f64 {
    if diag_weight() > 0.0 {
        return total();
    }
    total() + tunable!("ENOMOTO_T_TICKS_W_DIAG", 2.0, f64) * DIAG.with(|c| c.get()) as f64
}

/// このスレッドの部分ごとの手間 (診断用)。
pub fn raw() -> [u64; NK] {
    RAW.with(|r| std::array::from_fn(|k| r[k].get()))
}

/// 部分ごとの手間 `d` (別スレッドで測った [`raw`] の差) をこのスレッドに足す (別スレッドに任せて待った仕事の手間を呼び手に数える)。
pub fn add_raw(d: &[u64; NK]) {
    RAW.with(|r| {
        for k in 0..NK {
            r[k].set(r[k].get().wrapping_add(d[k]));
        }
    });
}

/// このスレッドに手間の門が付いているか ([`install_gate`])
pub fn has_gate() -> bool {
    HAS_GATE.with(|g| g.get())
}

/// このスレッドの手間の期限 (重み付きの合計) を過ぎたか (期限がなければ偽)。
#[inline]
pub fn over() -> bool {
    let t = total();
    if HAS_GATE.with(|g| g.get()) && gate_check(t) {
        return true;
    }
    t >= DEADLINE.with(|d| d.get())
}

/// 試作 `ENOMOTO_MIP_TICKS` の `j` (analysis/timedep_20261114.md の 12.5 節): このスレッドの時計を手間にするか ([`set_vclock`])。
/// 真なら、FJ・Local-MIP・旧来の局所探索の時間の上限・取り分を秒の代わりに手間 ([`total`] × 1e-9 秒) で数える
pub fn vclock() -> bool {
    VCLOCK.with(|v| v.get())
}

pub fn set_vclock(on: bool) {
    VCLOCK.with(|v| v.set(on));
}

/// [`vclock`] なら手間の秒 ([`total`] × 1e-9)、そうでなければ `t0` からの壁時計の秒 (経過の比べに使う。基準は呼ぶ側で引く)。
pub fn clock_secs(t0: std::time::Instant) -> f64 {
    if vclock() { total() * 1e-9 } else { t0.elapsed().as_secs_f64() }
}

/// 手間の門 (試作 `ENOMOTO_MIP_TICKS` の `j`): 別スレッド (FJ・Local-MIP) が許された手間 (`granted`、そのスレッドの [`total`]) に
/// 着いたら [`over`] の中で止まって待ち、主が [`Gate::wait_paused`] で止まったのを確かめてから受け渡し箱を読み、[`Gate::grant`] で
/// 次の手間を許す。スレッドの計算は許し方によらず同じで、主が読む時点のスレッドの状態も決まる (止まる点は手間で決まる)。
#[derive(Default)]
pub struct Gate {
    st: std::sync::Mutex<GateState>,
    cv: std::sync::Condvar,
    /// `st.granted` の写し (f64 のビット) と止めの合図 (スレッドの速い道: ロックを取らずに比べる)
    granted_bits: std::sync::atomic::AtomicU64,
    stopped: std::sync::atomic::AtomicBool,
}

#[derive(Default, Clone, Copy)]
pub struct GateState {
    pub granted: f64,
    /// 止まった (か終わった) ときのスレッドの手間
    pub progress: f64,
    pub paused: bool,
    pub done: bool,
    pub stop: bool,
}

impl Gate {
    pub fn new(granted: f64) -> std::sync::Arc<Gate> {
        std::sync::Arc::new(Gate {
            st: std::sync::Mutex::new(GateState { granted, ..Default::default() }),
            cv: std::sync::Condvar::new(),
            granted_bits: std::sync::atomic::AtomicU64::new(granted.to_bits()),
            stopped: std::sync::atomic::AtomicBool::new(false),
        })
    }
    /// 主: スレッドが門で止まるか終わるまで待ち、その時点の状態を返す
    pub fn wait_paused(&self) -> GateState {
        let mut g = self.st.lock().unwrap();
        while !(g.paused || g.done) {
            g = self.cv.wait(g).unwrap();
        }
        *g
    }
    /// 主: 許す手間を `add` 足す (止まっていれば動き出す)
    pub fn grant(&self, add: f64) {
        let mut g = self.st.lock().unwrap();
        g.granted += add.max(0.0);
        self.granted_bits.store(g.granted.to_bits(), std::sync::atomic::Ordering::Release);
        self.cv.notify_all();
    }
    /// 主: 止めの合図 (門で止まっているスレッドは [`over`] が真を返して抜ける)
    pub fn stop(&self) {
        let mut g = self.st.lock().unwrap();
        g.stop = true;
        self.stopped.store(true, std::sync::atomic::Ordering::Release);
        self.cv.notify_all();
    }
    /// スレッド: 終わった印 ([`GateGuard`] が捨てるときに呼ぶ)
    fn finish(&self) {
        let mut g = self.st.lock().unwrap();
        g.done = true;
        g.progress = total();
        self.cv.notify_all();
    }
}

/// スレッドに門を付ける ([`over`] で止まる)。捨てるときに門を外し、終わった印を付ける (途中で戻っても・panic しても)
pub struct GateGuard(std::sync::Arc<Gate>);

pub fn install_gate(g: std::sync::Arc<Gate>) -> GateGuard {
    GATE.with(|x| *x.borrow_mut() = Some(g.clone()));
    HAS_GATE.with(|h| h.set(true));
    GateGuard(g)
}

impl Drop for GateGuard {
    fn drop(&mut self) {
        GATE.with(|x| *x.borrow_mut() = None);
        HAS_GATE.with(|h| h.set(false));
        self.0.finish();
    }
}

/// 門を見る: 許された手間に着いていれば止まって待つ。止めの合図なら真 (手間の期限を過ぎたのと同じに扱う)
fn gate_check(t: f64) -> bool {
    let Some(gate) = GATE.with(|x| x.borrow().clone()) else { return false };
    // 速い道: まだ許された手間の内で、止めの合図もない
    if !gate.stopped.load(std::sync::atomic::Ordering::Acquire) && t < f64::from_bits(gate.granted_bits.load(std::sync::atomic::Ordering::Acquire)) {
        return false;
    }
    let mut g = gate.st.lock().unwrap();
    if g.stop {
        return true;
    }
    if t < g.granted {
        return false;
    }
    g.paused = true;
    g.progress = t;
    gate.cv.notify_all();
    while t >= g.granted && !g.stop {
        g = gate.cv.wait(g).unwrap();
    }
    g.paused = false;
    g.stop
}

/// このスレッドの手間の期限までの残り (重み付き、期限がなければ +inf)。
pub fn left() -> f64 {
    (DEADLINE.with(|d| d.get()) - total()).max(0.0)
}

/// 今から `budget` (重み付き) を手間の期限にして `f` を実行し、元に戻す (今の期限より早いときだけ縮める)。
pub fn with_budget<R>(budget: f64, f: impl FnOnce() -> R) -> R {
    let prev = DEADLINE.with(|d| d.get());
    DEADLINE.with(|d| d.set(prev.min(total() + budget)));
    let r = f();
    DEADLINE.with(|d| d.set(prev));
    r
}

/// 手間の期限を外して `f` を実行し、元に戻す (手間は数え続ける)。返す暫定解の研磨 (`crate::mip` の `polish_in_problem_warm`) の LP は、
/// 呼び出し元 (ヒューリスティクスなど) の手間の期限で数反復で止まると、研磨しないまま見積りで受け取ることになるので、壁時計の期限だけで止める
/// (analysis/nattai_20261125 の 9 節)
pub fn without_budget<R>(f: impl FnOnce() -> R) -> R {
    let prev = DEADLINE.with(|d| d.get());
    DEADLINE.with(|d| d.set(f64::INFINITY));
    let r = f();
    DEADLINE.with(|d| d.set(prev));
    r
}

/// 今から `budget` (重み付き) を手間の期限にする (今の期限より早いときだけ縮める)。戻り値の前の期限を [`pop_budget`] に渡して戻す。
pub fn push_budget(budget: f64) -> f64 {
    let prev = DEADLINE.with(|d| d.get());
    DEADLINE.with(|d| d.set(prev.min(total() + budget)));
    prev
}

/// [`push_budget`] の前の期限に戻す。
pub fn pop_budget(prev: f64) {
    DEADLINE.with(|d| d.set(prev));
}

/// このスレッドに手間の期限があるか。
pub fn has_budget() -> bool {
    DEADLINE.with(|d| d.get()).is_finite()
}

/// [`push_budget`] して、捨てるときに元の期限に戻す見張り (途中で戻る関数で使う)。
pub struct BudgetGuard(f64);

impl Drop for BudgetGuard {
    fn drop(&mut self) {
        pop_budget(self.0);
    }
}

/// 今から `budget` を手間の期限にし、戻り値を捨てるときに戻す。
pub fn budget_guard(budget: f64) -> BudgetGuard {
    BudgetGuard(push_budget(budget))
}

/// 部分 `k` の重み (ns / 手間)。
pub fn weight(k: usize) -> f64 {
    weights()[k]
}
