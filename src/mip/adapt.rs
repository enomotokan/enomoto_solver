//! 手間の配り方で共通に使う小さな関数 (T1PRIMAL v2 の取り分 [`super::t1primal`] と、試作 `ENOMOTO_MIP_DIVESEL` の `g`
//! (analysis/divebudget_20261127) で同じ式を使う)。
//! - [`laplace`]: 割合 x を回数 n でならす (x·n + 1)/(n + 2)。n = 0 では 1/2、n が増えると x に近づく。暫定解がない間のギャップ 1 も
//!   (n + 1)/(n + 2) < 1 になり、[`logit`] が +∞ に振り切れない。
//! - [`logit`]・[`sigmoid`]: 割合を対数オッズで足し合わせ、0〜1 に戻す。

/// 割合 `x` (0〜1) を回数 `n` でラプラスの式 (x·n + 1)/(n + 2) にならす
pub(crate) fn laplace(x: f64, n: f64) -> f64 {
    (x.clamp(0.0, 1.0) * n + 1.0) / (n + 2.0)
}

/// 対数オッズ ln(p / (1 − p)) (p は [1e-6, 1 − 1e-6] に収める)
pub(crate) fn logit(p: f64) -> f64 {
    let p = p.clamp(1e-6, 1.0 - 1e-6);
    (p / (1.0 - p)).ln()
}

pub(crate) fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laplace_never_saturates() {
        assert_eq!(laplace(1.0, 0.0), 0.5);
        assert!(laplace(1.0, 1e6) < 1.0);
        assert!(logit(laplace(1.0, 100.0)).is_finite());
        assert!((sigmoid(logit(0.3)) - 0.3).abs() < 1e-12);
    }
}
