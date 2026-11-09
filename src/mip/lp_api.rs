//! 分枝限定法が LP に求める操作 ([`MipLp`])。実装は
//! [`crate::simplex::mip_lp::TwoStageLp`] (既存の傾き・切片二段解法を warm start で使うもの)。

use super::lp::{Basis, LpStatus, SolveLimits, VarStatus};
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
    /// 行 `i` の係数を複製せずに見る ([`Self::row`] と同じ中身)
    fn row_ref(&self, i: usize) -> &[(usize, f64)];
    fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64);
    fn set_costs(&mut self, c: &[f64]);
    fn costs(&self) -> Vec<f64>;
    fn add_rows(&mut self, rows: &[(Vec<(usize, f64)>, f64, f64)]);
    fn delete_rows(&mut self, remove: &[bool]);
    /// 行を消してから行を加える ([`TwoStageLp::delete_and_add_rows`]、標準形の作り直しを 1 回にまとめる)。既定は順に呼ぶ。
    fn delete_and_add_rows(&mut self, remove: &[bool], rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        self.delete_rows(remove);
        self.add_rows(rows);
    }
    /// 行列 (行の集合) の識別子 ([`TwoStageLp::matrix_key`]。行が変わると変わる)。対応しない実装は `None`。
    fn matrix_key(&self) -> Option<u64> {
        None
    }
    fn basis(&self) -> Basis;
    fn set_basis(&mut self, b: &Basis);
    /// 試作 (`ENOMOTO_MIP_BASIS_NB`): 基底と一緒に非基底の上限・下限の置き場所も次の求解に渡す ([`TwoStageLp::set_basis_nb`])。
    /// 対応しない実装は [`Self::set_basis`] と同じ。
    fn set_basis_nb(&mut self, b: &Basis, _skip: &[usize]) {
        self.set_basis(b)
    }
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
    /// 根の LP の安全網: 尺度を調整した写しを基底なしで解いて元の LP を解き直す ([`TwoStageLp::solve_scaled_rescue`])。
    /// 試さない (係数の幅が狭い・対応しない実装) なら `None`。
    fn solve_scaled_rescue(&mut self, _lim: &SolveLimits) -> Option<LpStatus> {
        None
    }
    /// `ENOMOTO_MIP_ROOTLP_SCALE` (既定で使う): 次の求解 1 回だけ尺度を調整した写しで解く ([`TwoStageLp::request_scale_once`])。
    fn request_scale_once(&mut self) {}
    /// 試作 `ENOMOTO_MIP_PROBE_BASIS_GUARD` (analysis/probe_basis_20261106.md): 基底を捨てる ([`TwoStageLp::drop_basis`])。
    fn drop_basis(&mut self) {}
    /// 同: 基底があっても基底なしの内点法 + クロスオーバーと同時に解く ([`TwoStageLp::solve_race_warm`])。対応しない実装は [`Self::solve`]。
    fn solve_race_warm(&mut self, lim: &SolveLimits, _threads: usize) -> LpStatus {
        self.solve(lim)
    }
    /// 同: 今の基底の出発の点の性質 ([`TwoStageLp::basis_start_info`])。特異・対応しない実装は `None`。
    fn basis_start_info(&self, _col: &[VarStatus]) -> Option<(usize, f64)> {
        None
    }
    /// 試作 `ENOMOTO_MIP_OPTFACE_MOVE` ([`TwoStageLp::optface_move`]): 最適面の上の別の最適基底へ移る。対応しない実装は `None`。
    fn optface_move(&mut self, _k: usize, _sample: usize, _mode: char, _seed: u64, _is_int: &[bool], _deadline: Option<std::time::Instant>) -> Option<crate::simplex::mip_lp::OptFaceStats> {
        None
    }
    /// 試作 `ENOMOTO_MIP_ROOTLP_BOTH`: 根の LP の同時実行の負けた側の LP ([`crate::simplex::mip_lp::BothLoser`]) をこの LP に
    /// 置き換える。対応しない実装は `false`。
    fn adopt_two_stage(&mut self, _lp: TwoStageLp) -> bool {
        false
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
    /// 今の基底 (基底列の番号、[`TwoStageLp::basis_cols`])。対応しない実装は `None`。
    fn basis_cols(&self) -> Option<Vec<usize>> {
        None
    }
    /// 診断用: 今の LP と出発の基底をファイルに書き出す ([`TwoStageLp::dump_lp`])。対応しない実装は何もしない。
    fn dump_lp(&self, _prefix: &str, _basis: Option<&[usize]>, _weights: Option<&[f64]>) -> std::io::Result<()> {
        Ok(())
    }
    /// 次の求解に渡す DSE 重み (基底位置順、[`TwoStageLp::dse_weights`])。対応しない実装は `None`。
    fn dse_weights(&self) -> Option<Vec<f64>> {
        None
    }
    /// 診断用: 今の LP を新しい LP として同じ基底から解き直す ([`TwoStageLp::resolve_fresh_debug`])。対応しない実装は何もしない。
    fn resolve_fresh_debug(&self, _basis: Option<&[usize]>, _weights: Option<&[f64]>) {}
    /// 診断用: 引き継ぐ DSE 重みと厳密な重みを比べて表示する ([`TwoStageLp::debug_dse_check`])。対応しない実装は何もしない。
    fn debug_dse_check(&self, _tag: &str) {}
    /// 今の基底の DSE 重み (基底変数の番号との組、[`TwoStageLp::dse_snapshot`])。
    fn dse_snapshot(&self) -> Option<Vec<(u32, f32)>>;
    /// DSE 重みを今の基底に当てはめる ([`TwoStageLp::set_dse_vars`])。
    fn set_dse_vars(&mut self, w: &[(u32, f32)]);
    /// 今の基底の DSE 重みの育ち具合 ([`TwoStageLp::dse_age`])。対応しない実装は `None`。
    fn dse_age(&self) -> Option<u64> {
        None
    }
    /// 今の基底の厳密な DSE 重みを計算し直す ([`TwoStageLp::recompute_dse_exact`])。対応しない実装は `None`。
    fn recompute_dse_exact(&mut self, _stop: &dyn Fn() -> bool) -> Option<bool> {
        None
    }
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
            fn row_ref(&self, i: usize) -> &[(usize, f64)] { <$t>::row_ref(self, i) }
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
    fn solve_scaled_rescue(&mut self, lim: &SolveLimits) -> Option<LpStatus> {
        TwoStageLp::solve_scaled_rescue(self, lim)
    },
    fn request_scale_once(&mut self) {
        TwoStageLp::request_scale_once(self)
    },
    fn drop_basis(&mut self) {
        TwoStageLp::drop_basis(self)
    },
    fn solve_race_warm(&mut self, lim: &SolveLimits, threads: usize) -> LpStatus {
        TwoStageLp::solve_race_warm(self, lim, threads)
    },
    fn basis_start_info(&self, col: &[VarStatus]) -> Option<(usize, f64)> {
        TwoStageLp::basis_start_info(self, col)
    },
    fn optface_move(&mut self, k: usize, sample: usize, mode: char, seed: u64, is_int: &[bool], deadline: Option<std::time::Instant>) -> Option<crate::simplex::mip_lp::OptFaceStats> {
        TwoStageLp::optface_move(self, k, sample, mode, seed, is_int, deadline)
    },
    fn adopt_two_stage(&mut self, lp: TwoStageLp) -> bool {
        *self = lp;
        true
    },
    fn set_basis_cols(&mut self, b: &[usize], x: Option<&[f64]>) {
        TwoStageLp::set_basis_cols(self, b, x)
    },
    fn set_basis_nb(&mut self, b: &Basis, skip: &[usize]) {
        TwoStageLp::set_basis_nb(self, b, skip)
    },
    fn basis_cols(&self) -> Option<Vec<usize>> {
        TwoStageLp::basis_cols(self)
    },
    fn dump_lp(&self, prefix: &str, basis: Option<&[usize]>, weights: Option<&[f64]>) -> std::io::Result<()> {
        TwoStageLp::dump_lp(self, prefix, basis, weights)
    },
    fn dse_weights(&self) -> Option<Vec<f64>> {
        TwoStageLp::dse_weights(self)
    },
    fn resolve_fresh_debug(&self, basis: Option<&[usize]>, weights: Option<&[f64]>) {
        TwoStageLp::resolve_fresh_debug(self, basis, weights)
    },
    fn debug_dse_check(&self, tag: &str) {
        TwoStageLp::debug_dse_check(self, tag)
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
    fn dse_age(&self) -> Option<u64> {
        TwoStageLp::dse_age(self)
    },
    fn recompute_dse_exact(&mut self, stop: &dyn Fn() -> bool) -> Option<bool> {
        TwoStageLp::recompute_dse_exact(self, stop)
    },
    fn lu_snapshot(&self) -> Option<LuSnapshot> {
        TwoStageLp::lu_snapshot(self)
    },
    fn adopt_lu_snapshot(&mut self, s: &LuSnapshot) -> bool {
        TwoStageLp::adopt_lu_snapshot(self, s)
    },
    fn delete_and_add_rows(&mut self, remove: &[bool], rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        TwoStageLp::delete_and_add_rows(self, remove, rows)
    },
    fn matrix_key(&self) -> Option<u64> {
        Some(TwoStageLp::matrix_key(self))
    }
);
