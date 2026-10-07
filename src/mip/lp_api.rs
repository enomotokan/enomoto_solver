//! 分枝限定法が LP に求める操作 ([`MipLp`])。実装は 2 つ:
//! [`super::lp::LpEngine`] (分枝限定法専用に書いた単体法) と
//! [`crate::simplex::mip_lp::TwoStageLp`] (既存の傾き・切片二段解法を warm start で使うもの)。

use super::lp::{Basis, LpEngine, LpState, LpStatus, SolveLimits};
use crate::simplex::mip_lp::{TwoStageLp, TwoStageState};

/// 分枝限定法が使う LP の操作。
pub trait MipLp: Clone {
    type State: Clone;
    fn new(col_lo: &[f64], col_up: &[f64], cost: &[f64], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self;
    fn num_rows(&self) -> usize;
    fn total_iterations(&self) -> u64;
    fn col_bounds(&self, j: usize) -> (f64, f64);
    fn row_bounds(&self, i: usize) -> (f64, f64);
    fn row(&self, i: usize) -> Vec<(usize, f64)>;
    fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64);
    fn set_costs(&mut self, c: &[f64]);
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
    fn objective(&self) -> f64;
    fn col_values(&self) -> Vec<f64>;
    fn col_value(&self, j: usize) -> f64;
    fn row_activities(&self) -> Vec<f64>;
    fn reduced_costs(&self) -> Vec<f64>;
    fn row_duals(&self) -> Vec<f64>;
    fn basis_inverse_row(&mut self, s: usize) -> Vec<f64>;
}

macro_rules! forward_impl {
    ($t:ty, $st:ty) => {
        impl MipLp for $t {
            type State = $st;
            fn new(col_lo: &[f64], col_up: &[f64], cost: &[f64], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self {
                <$t>::new(col_lo, col_up, cost, rows, row_lo, row_up)
            }
            fn num_rows(&self) -> usize { <$t>::num_rows(self) }
            fn total_iterations(&self) -> u64 { <$t>::total_iterations(self) }
            fn col_bounds(&self, j: usize) -> (f64, f64) { <$t>::col_bounds(self, j) }
            fn row_bounds(&self, i: usize) -> (f64, f64) { <$t>::row_bounds(self, i) }
            fn row(&self, i: usize) -> Vec<(usize, f64)> { <$t>::row(self, i) }
            fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64) { <$t>::set_col_bounds(self, j, lo, up) }
            fn set_costs(&mut self, c: &[f64]) { <$t>::set_costs(self, c) }
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
            fn basis_inverse_row(&mut self, s: usize) -> Vec<f64> { <$t>::basis_inverse_row(self, s) }
        }
    };
}

forward_impl!(LpEngine, LpState);
forward_impl!(TwoStageLp, TwoStageState);
