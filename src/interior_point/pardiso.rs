//! Intel MKL PARDISO による正規方程式 (対称正定値) の疎 Cholesky 分解 (試験用、`ENOMOTO_T_CHOL_BACKEND=1`)。
//!
//! MKL は実行時に動的に読み込む (`ENOMOTO_MKL_RT` のパス、なければ `libmkl_rt.so.3`・`.so.2`・`.so`)。読み込めなければ
//! [`Pardiso::new`] は `None` を返し、呼び出し側は faer の分解を使う。並べ替えは MKL の並列 nested dissection
//! (`iparm[1] = 3`)、記号解析 (phase 11) は作るときに 1 回だけ、毎回は数値分解 (phase 22) と求解 (phase 33)。
//! スレッド数は呼び出したスレッドの rayon のプールの大きさに合わせる (`MKL_Set_Num_Threads_Local`)。

use std::ffi::c_void;
use std::sync::OnceLock;

type PardisoFn = unsafe extern "C" fn(
    pt: *mut *mut c_void,
    maxfct: *const i32,
    mnum: *const i32,
    mtype: *const i32,
    phase: *const i32,
    n: *const i32,
    a: *const f64,
    ia: *const i32,
    ja: *const i32,
    perm: *mut i32,
    nrhs: *const i32,
    iparm: *mut i32,
    msglvl: *const i32,
    b: *mut f64,
    x: *mut f64,
    error: *mut i32,
);
type SetThreadsLocalFn = unsafe extern "C" fn(nt: i32) -> i32;
type SetInterfaceFn = unsafe extern "C" fn(layer: i32) -> i32;

struct Mkl {
    _lib: libloading::Library,
    pardiso: PardisoFn,
    set_threads_local: SetThreadsLocalFn,
}

// SAFETY: 関数ポインタとライブラリの持ち手だけで、MKL の関数はスレッドから呼べる。
unsafe impl Send for Mkl {}
unsafe impl Sync for Mkl {}

fn mkl() -> Option<&'static Mkl> {
    static MKL: OnceLock<Option<Mkl>> = OnceLock::new();
    MKL.get_or_init(|| {
        let mut names: Vec<String> = Vec::new();
        if let Some(p) = env_str!("ENOMOTO_MKL_RT") {
            names.push(p.to_string());
        }
        names.extend(["libmkl_rt.so.3", "libmkl_rt.so.2", "libmkl_rt.so"].iter().map(|s| s.to_string()));
        for name in names {
            // SAFETY: MKL の初期化子を走らせるだけ。
            let Ok(lib) = (unsafe { libloading::Library::new(&name) }) else { continue };
            // SAFETY: シンボルの型は MKL の C の宣言どおり (LP64)。
            unsafe {
                let Ok(p) = lib.get::<PardisoFn>(b"pardiso\0") else { continue };
                let Ok(t) = lib.get::<SetThreadsLocalFn>(b"MKL_Set_Num_Threads_Local\0") else { continue };
                if let Ok(si) = lib.get::<SetInterfaceFn>(b"MKL_Set_Interface_Layer\0") {
                    (*si)(0); // LP64 (32 ビット整数)
                }
                let (p, t) = (*p, *t);
                return Some(Mkl { _lib: lib, pardiso: p, set_threads_local: t });
            }
        }
        None
    })
    .as_ref()
}

/// MKL PARDISO が使えるか。
pub fn available() -> bool {
    mkl().is_some()
}

/// 対称正定値行列 (上三角、列圧縮の非零の形で与える) の PARDISO による分解。
pub struct Pardiso {
    mkl: &'static Mkl,
    pt: [*mut c_void; 64],
    iparm: [i32; 64],
    n: i32,
    /// 上三角の行圧縮 (0 始まり)。
    ia: Vec<i32>,
    ja: Vec<i32>,
    a: Vec<f64>,
    /// 列圧縮の位置 `k` の値が行圧縮のどこへ行くか。
    to_csr: Vec<usize>,
    factored: bool,
    x: Vec<f64>,
}

// SAFETY: `pt` は PARDISO の内部の持ち手で、同時に 2 つのスレッドから使わない (`&mut self` でだけ触る)。
unsafe impl Send for Pardiso {}

impl Pardiso {
    /// 上三角の列圧縮の非零の形 (`col_ptrs`・`row_indices`、各列の行は昇順で対角を含む) から記号解析まで行う。
    pub fn new(n: usize, col_ptrs: &[usize], row_indices: &[usize]) -> Option<Self> {
        let mkl = mkl()?;
        if n == 0 || n > i32::MAX as usize || row_indices.len() > i32::MAX as usize {
            return None;
        }
        // 列圧縮 (上三角: 列 c の行 r <= c) → 行圧縮 (上三角: 行 r の列 c >= r)。列の順に積むので各行の列は昇順。
        let nnz = row_indices.len();
        let mut cnt = vec![0usize; n + 1];
        for &r in row_indices {
            cnt[r + 1] += 1;
        }
        for i in 0..n {
            cnt[i + 1] += cnt[i];
        }
        let ia: Vec<i32> = cnt.iter().map(|&v| v as i32).collect();
        let mut fill = cnt[..n].to_vec();
        let mut ja = vec![0i32; nnz];
        let mut to_csr = vec![0usize; nnz];
        for c in 0..n {
            for k in col_ptrs[c]..col_ptrs[c + 1] {
                let r = row_indices[k];
                let q = fill[r];
                fill[r] += 1;
                ja[q] = c as i32;
                to_csr[k] = q;
            }
        }
        let mut iparm = [0i32; 64];
        iparm[0] = 1; // 既定以外の値を使う
        iparm[1] = tunable!("ENOMOTO_T_PARDISO_ORDER", 3i32, i32); // 3 = 並列 nested dissection、2 = METIS、0 = 最小次数
        iparm[34] = 1; // 0 始まりの添字
        let mut s = Pardiso {
            mkl,
            pt: [std::ptr::null_mut(); 64],
            iparm,
            n: n as i32,
            ia,
            ja,
            a: vec![0.0; nnz],
            to_csr,
            factored: false,
            x: vec![0.0; n],
        };
        // 記号解析には値が要らないが、対角を正にしておく。
        for r in 0..n {
            let q = s.ia[r] as usize;
            s.a[q] = 1.0;
        }
        if s.call(11, None) != 0 {
            return None;
        }
        Some(s)
    }

    fn call(&mut self, phase: i32, rhs: Option<&mut [f64]>) -> i32 {
        let (maxfct, mnum, mtype, msglvl) = (1i32, 1i32, 2i32, 0i32);
        let nrhs = 1i32;
        let mut perm = 0i32;
        let mut error = 0i32;
        let nt = rayon::current_num_threads().max(1) as i32;
        let (b, x) = match rhs {
            Some(r) => (r.as_mut_ptr(), self.x.as_mut_ptr()),
            None => (std::ptr::null_mut(), std::ptr::null_mut()),
        };
        // SAFETY: 配列の長さは PARDISO の要求どおり (ia は n+1、ja・a は nnz、b・x は n)。
        unsafe {
            (self.mkl.set_threads_local)(nt);
            (self.mkl.pardiso)(
                self.pt.as_mut_ptr(),
                &maxfct,
                &mnum,
                &mtype,
                &phase,
                &self.n,
                self.a.as_ptr(),
                self.ia.as_ptr(),
                self.ja.as_ptr(),
                &mut perm,
                &nrhs,
                self.iparm.as_mut_ptr(),
                &msglvl,
                b,
                x,
                &mut error,
            );
            (self.mkl.set_threads_local)(0);
        }
        error
    }

    /// 上三角の列圧縮の値 (`new` に渡した形の順) で数値分解する。失敗 (正定値でない など) なら `false`。
    pub fn factor(&mut self, csc_values: &[f64]) -> bool {
        for (k, &v) in csc_values.iter().enumerate() {
            self.a[self.to_csr[k]] = v;
        }
        self.factored = self.call(22, None) == 0;
        self.factored
    }

    /// `rhs` を解で上書きする。
    pub fn solve_in_place(&mut self, rhs: &mut [f64]) {
        let err = self.call(33, Some(rhs));
        if err == 0 {
            rhs.copy_from_slice(&self.x);
        } else {
            rhs.fill(f64::NAN);
        }
    }

    /// 因子の非零の数 (PARDISO の報告)。
    pub fn factor_nnz(&self) -> usize {
        self.iparm[17].max(0) as usize
    }
}

impl Drop for Pardiso {
    fn drop(&mut self) {
        let _ = self.call(-1, None);
    }
}
