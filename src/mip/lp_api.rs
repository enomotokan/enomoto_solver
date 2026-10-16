//! 分枝限定法が LP に求める操作 ([`MipLp`])。実装は
//! [`crate::simplex::mip_lp::TwoStageLp`] (既存の傾き・切片二段解法を warm start で使うもの)。

use super::lp::{Basis, LpStatus, SolveLimits};
use crate::simplex::mip_lp::{InteriorPoint, LuSnapshot, TwoStageLp, TwoStageState};
use std::borrow::Cow;

/// 分枝限定法が使う LP の操作。
pub trait MipLp: Clone + Send {
    type State: Clone + Send;
    fn new(col_lo: &[f64], col_up: &[f64], cost: &[f64], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self;
    fn num_rows(&self) -> usize;
    fn total_iterations(&self) -> u64;
    /// 反復の累計を書き戻す (予算の数え方に入れたくない求解の分を戻す)。
    fn set_total_iterations(&mut self, v: u64);
    fn col_bounds(&self, j: usize) -> (f64, f64);
    fn row_bounds(&self, i: usize) -> (f64, f64);
    fn row(&self, i: usize) -> Vec<(usize, f64)>;
    fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64);
    fn set_costs(&mut self, c: &[f64]);
    fn costs(&self) -> Vec<f64>;
    fn add_rows(&mut self, rows: &[(Vec<(usize, f64)>, f64, f64)]);
    fn delete_rows(&mut self, remove: &[bool]);
    fn basis(&self) -> Basis;
    fn set_basis(&mut self, b: &Basis);
    fn basic_var(&self, s: usize) -> usize;
    fn dse_weight(&self, s: usize) -> f64;
    fn save_state(&self) -> Self::State;
    fn restore_state(&mut self, s: &Self::State);
    fn solve(&mut self, lim: &SolveLimits) -> LpStatus;
    fn solve_primal(&mut self, lim: &SolveLimits) -> LpStatus;
    /// [`Self::solve`] が結論を出せなかった (`Error`・`IterationLimit`) LP を別の経路で解き直す
    /// ([`TwoStageLp::solve_ipm_rescue`])。対応しない実装は `Error`。
    fn solve_rescue(&mut self, _lim: &SolveLimits) -> LpStatus {
        LpStatus::Error
    }
    fn objective(&self) -> f64;
    fn col_values(&self) -> Vec<f64>;
    fn col_value(&self, j: usize) -> f64;
    fn row_activities(&self) -> Vec<f64>;
    fn reduced_costs(&self) -> Vec<f64>;
    fn row_duals(&self) -> Vec<f64>;
    /// [`Self::basis`] を確保なしで見る (覚えておける実装は参照を返す。既定は作って返す)。
    fn basis_ref(&self) -> Cow<'_, Basis> {
        Cow::Owned(self.basis())
    }
    /// [`Self::row_activities`] を確保なしで見る (既定は作って返す)。
    fn row_activities_ref(&self) -> Cow<'_, [f64]> {
        Cow::Owned(self.row_activities())
    }
    /// [`Self::reduced_costs`] を確保なしで見る (既定は作って返す)。
    fn reduced_costs_ref(&self) -> Cow<'_, [f64]> {
        Cow::Owned(self.reduced_costs())
    }
    /// 直前の求解が実行不能だったときの双対射線 (元の行の向き、行の重み)。なければ `None`。
    fn farkas_ray(&self) -> Option<Vec<f64>>;
    fn basis_inverse_row(&mut self, s: usize) -> Vec<f64>;
    /// 今の LP を内点法で解いた点と、クロスオーバーの押し出しの頂点・基底 ([`TwoStageLp::interior_point`])。
    /// 基底は双対実行可能とは限らない。
    fn interior_point(&self, zero_cost: bool, want_vertex: bool, deadline: std::time::Instant) -> Option<InteriorPoint>;
    /// 直前の求解が非有界だったときの切片問題の境界 ([`TwoStageLp::intercept_bounds`])。
    fn intercept_bounds(&self) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)>;
    /// 基底を列番号 (構造列 j < n、行 n + i) で与える ([`TwoStageLp::set_basis_cols`])。
    fn set_basis_cols(&mut self, b: &[usize], x: Option<&[f64]>);
    /// 今の基底の DSE 重み (基底変数の番号との組、[`TwoStageLp::dse_snapshot`])。
    fn dse_snapshot(&self) -> Option<Vec<(u32, f32)>>;
    /// DSE 重みを今の基底に当てはめる ([`TwoStageLp::set_dse_vars`])。
    fn set_dse_vars(&mut self, w: &[(u32, f32)]);
    /// 今の基底の LU キャッシュを取り出す ([`TwoStageLp::lu_snapshot`])。対応しない実装は `None`。
    fn lu_snapshot(&self) -> Option<LuSnapshot> {
        None
    }
    /// 取り出した LU キャッシュを、今の基底が同じなら戻す ([`TwoStageLp::adopt_lu_snapshot`])。対応しない実装は何もしない。
    fn adopt_lu_snapshot(&mut self, _s: &LuSnapshot) -> bool {
        false
    }
}

macro_rules! forward_impl {
    ($t:ty, $st:ty $(, $extra:item)*) => {
        impl MipLp for $t {
            $($extra)*
            type State = $st;
            fn new(col_lo: &[f64], col_up: &[f64], cost: &[f64], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self {
                <$t>::new(col_lo, col_up, cost, rows, row_lo, row_up)
            }
            fn num_rows(&self) -> usize { <$t>::num_rows(self) }
            fn total_iterations(&self) -> u64 { <$t>::total_iterations(self) }
            fn set_total_iterations(&mut self, v: u64) { <$t>::set_total_iterations(self, v) }
            fn col_bounds(&self, j: usize) -> (f64, f64) { <$t>::col_bounds(self, j) }
            fn row_bounds(&self, i: usize) -> (f64, f64) { <$t>::row_bounds(self, i) }
            fn row(&self, i: usize) -> Vec<(usize, f64)> { <$t>::row(self, i) }
            fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64) { <$t>::set_col_bounds(self, j, lo, up) }
            fn set_costs(&mut self, c: &[f64]) { <$t>::set_costs(self, c) }
            fn costs(&self) -> Vec<f64> { <$t>::costs(self) }
            fn add_rows(&mut self, rows: &[(Vec<(usize, f64)>, f64, f64)]) { <$t>::add_rows(self, rows) }
            fn delete_rows(&mut self, remove: &[bool]) { <$t>::delete_rows(self, remove) }
            fn basis(&self) -> Basis { <$t>::basis(self) }
            fn set_basis(&mut self, b: &Basis) { <$t>::set_basis(self, b) }
            fn basic_var(&self, s: usize) -> usize { <$t>::basic_var(self, s) }
            fn dse_weight(&self, s: usize) -> f64 { <$t>::dse_weight(self, s) }
            fn save_state(&self) -> Self::State { <$t>::save_state(self) }
            fn restore_state(&mut self, s: &Self::State) { <$t>::restore_state(self, s) }
            fn solve(&mut self, lim: &SolveLimits) -> LpStatus { <$t>::solve(self, lim) }
            fn solve_primal(&mut self, lim: &SolveLimits) -> LpStatus { <$t>::solve_primal(self, lim) }
            fn objective(&self) -> f64 { <$t>::objective(self) }
            fn col_values(&self) -> Vec<f64> { <$t>::col_values(self) }
            fn col_value(&self, j: usize) -> f64 { <$t>::col_value(self, j) }
            fn row_activities(&self) -> Vec<f64> { <$t>::row_activities(self) }
            fn reduced_costs(&self) -> Vec<f64> { <$t>::reduced_costs(self) }
            fn row_duals(&self) -> Vec<f64> { <$t>::row_duals(self) }
            fn farkas_ray(&self) -> Option<Vec<f64>> { <$t>::farkas_ray(self) }
            fn basis_inverse_row(&mut self, s: usize) -> Vec<f64> { <$t>::basis_inverse_row(self, s) }
        }
    };
}

forward_impl!(
    TwoStageLp,
    TwoStageState,
    fn basis_ref(&self) -> Cow<'_, Basis> {
        Cow::Borrowed(TwoStageLp::basis_ref(self))
    },
    fn row_activities_ref(&self) -> Cow<'_, [f64]> {
        Cow::Borrowed(TwoStageLp::row_activities_ref(self))
    },
    fn reduced_costs_ref(&self) -> Cow<'_, [f64]> {
        Cow::Borrowed(TwoStageLp::reduced_costs_ref(self))
    },
    fn interior_point(&self, zero_cost: bool, want_vertex: bool, deadline: std::time::Instant) -> Option<InteriorPoint> {
        TwoStageLp::interior_point(self, zero_cost, want_vertex, deadline)
    },
    fn solve_rescue(&mut self, lim: &SolveLimits) -> LpStatus {
        TwoStageLp::solve_ipm_rescue(self, lim)
    },
    fn set_basis_cols(&mut self, b: &[usize], x: Option<&[f64]>) {
        TwoStageLp::set_basis_cols(self, b, x)
    },
    fn intercept_bounds(&self) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
        TwoStageLp::intercept_bounds(self)
    },
    fn dse_snapshot(&self) -> Option<Vec<(u32, f32)>> {
        TwoStageLp::dse_snapshot(self)
    },
    fn set_dse_vars(&mut self, w: &[(u32, f32)]) {
        TwoStageLp::set_dse_vars(self, w)
    },
    fn lu_snapshot(&self) -> Option<LuSnapshot> {
        TwoStageLp::lu_snapshot(self)
    },
    fn adopt_lu_snapshot(&mut self, s: &LuSnapshot) -> bool {
        TwoStageLp::adopt_lu_snapshot(self, s)
    }
);
