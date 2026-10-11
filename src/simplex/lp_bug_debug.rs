//! 既存 LP 経路の誤答を調べるための診断テスト (`#[ignore]`)。
//! `LPBUG=tests/data/lp_bugs/xxx.txt cargo test --release --lib lp_bug_debug -- --ignored --nocapture`

use super::*;
use crate::types::{LinearExpr, LpOptions, VarType};

pub(crate) fn load(path: &str) -> (Vec<VariableData>, Objective, Vec<ConstraintRow>) {
    let s = std::fs::read_to_string(path).unwrap();
    let mut lines = s.lines();
    let mut hd = lines.next().unwrap().split_whitespace().map(|t| t.parse::<usize>().unwrap());
    let (n, m) = (hd.next().unwrap(), hd.next().unwrap());
    let p = |t: &str| -> f64 { t.parse().unwrap() };
    let mut vars = Vec::new();
    let mut coeffs = std::collections::BTreeMap::new();
    for j in 0..n {
        let t: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
        vars.push(VariableData { vtype: VarType::Continuous, lb: p(t[0]), ub: p(t[1]) });
        coeffs.insert(j, p(t[2]));
    }
    let mut cons = Vec::new();
    for _ in 0..m {
        let t: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
        let (lo, up, k) = (p(t[0]), p(t[1]), t[2].parse::<usize>().unwrap());
        let mut e = std::collections::BTreeMap::new();
        for q in 0..k {
            e.insert(t[3 + 2 * q].parse::<usize>().unwrap(), p(t[4 + 2 * q]));
        }
        let expr = LinearExpr { coeffs: e, constant: 0.0 };
        if lo == up {
            cons.push(ConstraintRow { expr, sense: RowSense::Eq, rhs: lo });
        } else {
            if lo.is_finite() {
                cons.push(ConstraintRow { expr: expr.clone(), sense: RowSense::Ge, rhs: lo });
            }
            if up.is_finite() {
                cons.push(ConstraintRow { expr, sense: RowSense::Le, rhs: up });
            }
        }
    }
    (vars, Objective { expr: LinearExpr { coeffs, constant: 0.0 }, sense: Sense::Minimize }, cons)
}

#[test]
#[ignore]
fn lp_bug_debug() {
    let path = std::env::var("LPBUG").unwrap();
    let (vars, obj, cons) = load(&path);
    match build_std_form_presolved(&vars, &obj, &cons, false) {
        Err(s) => eprintln!("presolve verdict: {s:?}"),
        Ok(pf) => {
            let std = &pf.std;
            eprintln!("presolved std: n_total={} n_rows={}", std.n_total, std.n_rows);
            let opts = LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
            let r = slope_intercept_dual::solve_slope_intercept_dual(std, &opts);
            match r {
                Some(r) => {
                    eprintln!("presolved (two-stage): {:?}", r.status);
                    if let Some(x) = &r.x {
                        let o: f64 = (0..std.n_total).map(|k| std.c[k] * x[k]).sum();
                        let mut maxv = 0.0f64;
                        for i in 0..std.n_rows {
                            let a: f64 = std.rows.row(i).iter().map(|&(k, v)| v * x[k]).sum();
                            maxv = maxv.max((a - std.b[i]).abs());
                        }
                        let mut maxb = 0.0f64;
                        for k in 0..std.n_total {
                            maxb = maxb.max(std.lb[k] - x[k]).max(x[k] - std.ub[k]);
                        }
                        eprintln!("   obj {o} max row residual {maxv} max bound viol {maxb}");
                    }
                }
                None => eprintln!("presolved (two-stage): None"),
            }
        }
    }
}

/// 回帰テスト: 固定列を畳んだ後に 1 変数になった行が既存の境界と矛盾する問題を、doubleton が
/// min/max で並べ替えて実行可能に変えていた (2026-10 修正)。
#[test]
fn doubleton_keeps_contradictory_bounds_infeasible() {
    let (vars, obj, cons) = load("tests/data/lp_bugs/infeasible_reported_optimal.txt");
    let opts = LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
    let r = crate::solver::solve_lp(&vars, &obj, &cons, crate::types::RootSolver::Simplex, opts);
    assert_eq!(r.status, Status::Infeasible);
}

/// 回帰テスト: dualpropagate が非有界な LP を「最適」「実行不能」に変えていた (既定で無効化)。
#[test]
fn unbounded_cases_reported_unbounded() {
    for f in ["tests/data/lp_bugs/unbounded_reported_optimal.txt", "tests/data/lp_bugs/unbounded_reported_infeasible.txt"] {
        let (vars, obj, cons) = load(f);
        let opts = LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
        let r = crate::solver::solve_lp(&vars, &obj, &cons, crate::types::RootSolver::Simplex, opts);
        assert_eq!(r.status, Status::Unbounded, "{f}");
    }
}
