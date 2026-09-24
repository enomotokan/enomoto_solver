//! 内点法 (PIQP 型) が前提とする二次計画の標準形
//!
//!   minimize   0.5 x^T P x + c^T x
//!   subject to A x = b
//!              G x <= h
//!
//! を、モデルの変数・目的関数・制約から直接組み立てる (LP なので P = 0)。
//! 内点法は任意の境界をそのまま扱えるので、変数のシフトや分割はしない。
//! 変数境界は `G` の追加行になる (共通の [`crate::presolve::build_a_g`] を使用)。
//! `A` と `G` は faer の CSR (`SparseRowMat`) のまま最後まで疎で扱う。

use super::kkt::Csr;
use crate::presolve::build_a_g;
use crate::types::{ConstraintRow, VariableData};

/// 内点法用の標準形 `min c^T x  s.t.  A x = b,  G x <= h`。
pub struct QpStd {
    /// 変数の数。
    pub n: usize,
    /// 目的関数の係数 (最小化形)。
    pub c: Vec<f64>,
    /// 等式制約の係数行列。
    pub a: Csr,
    /// 等式制約の右辺。
    pub b: Vec<f64>,
    /// 不等式制約 (変数境界を含む) の係数行列。
    pub g: Csr,
    /// 不等式制約の右辺。
    pub h: Vec<f64>,
}

/// 変数・最小化形の目的係数 `(変数番号, 係数)`・制約から `QpStd` を組み立てる。
/// 同じ変数番号の係数は合算する。
pub fn build(variables: &[VariableData], obj_coeffs_for_min: &[(usize, f64)], constraints: &[ConstraintRow]) -> QpStd {
    let n = variables.len();

    let mut c = vec![0.0; n];
    for &(j, v) in obj_coeffs_for_min {
        c[j] += v;
    }

    let (a, b, g, h) = build_a_g(variables, constraints);

    QpStd { n, c, a, b, g, h }
}
