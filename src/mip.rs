//! Simple depth-first branch-and-bound for models that contain integer or
//! binary variables. Each node re-solves the LP relaxation with tightened
//! variable bounds (no warm start) — simple and correct, adequate for the
//! problem sizes this MVP targets.

use crate::solver::solve_lp;
use crate::types::{ConstraintRow, Objective, RootSolver, Sense, SolveResult, Status, VarType, VariableData};
use std::collections::HashMap;

/// How close to an integer a discrete variable's LP-relaxation value must
/// be to count as "already integer" (`most_fractional` below).
const INT_TOL: f64 = 1e-6;
/// How much better a candidate objective must be than the current
/// incumbent to replace it — guards against replacing the incumbent over
/// and over for a difference that's really just floating-point noise.
const OBJ_EPS: f64 = 1e-7;
/// Safety cap on the number of branch-and-bound nodes explored; if hit,
/// `solve_mip` returns the best incumbent found so far with
/// `node_limit_hit: true` rather than the (unproven) true optimum.
const MAX_NODES: usize = 20_000;

/// One node of the branch-and-bound tree: the original problem with some
/// variables' bounds further tightened. `overrides[j] = (lo, hi)` means
/// variable `j`'s bounds are additionally restricted to `[lo, hi]` at
/// this node (intersected with its original bounds when the node is
/// processed) — variables with no entry keep their original bounds.
struct Node {
    overrides: HashMap<usize, (f64, f64)>,
}

/// True if `candidate` is a strict improvement over `current` for the
/// given optimization `sense`, beyond the `OBJ_EPS` noise floor.
fn is_better(sense: Sense, candidate: f64, current: f64) -> bool {
    match sense {
        Sense::Minimize => candidate < current - OBJ_EPS,
        Sense::Maximize => candidate > current + OBJ_EPS,
    }
}

/// The index of the discrete (`Integer`) variable whose LP-relaxation
/// value `x[j]` is *most* fractional (closest to a half-integer), or
/// `None` if every discrete variable is already within `INT_TOL` of an
/// integer — i.e. `None` means this relaxation's solution is MIP-feasible
/// as-is. Picking the most-fractional variable to branch on (rather than,
/// say, the first fractional one) tends to shrink the remaining search
/// space fastest since it's the variable the relaxation is "most unsure"
/// about.
fn most_fractional(variables: &[VariableData], x: &[f64]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (j, v) in variables.iter().enumerate() {
        if v.vtype == VarType::Continuous {
            continue;
        }
        let val = x[j];
        let frac = val - val.floor();
        let score = frac.min(1.0 - frac);
        if score > INT_TOL {
            if best.map_or(true, |(_, bs)| score > bs) {
                best = Some((j, score));
            }
        }
    }
    best.map(|(j, _)| j)
}

/// Solves a (possibly mixed-integer) LP by depth-first branch-and-bound.
/// Falls straight through to a single `solve_lp` call when there are no
/// discrete variables at all. Otherwise: repeatedly pops a node, re-solves
/// its LP relaxation (`solve_lp`, no warm start — see the module docs),
/// prunes it if the relaxation is itself infeasible or can't beat the
/// current incumbent, and otherwise either accepts it as a new incumbent
/// (every discrete variable already integer) or branches on the most
/// fractional discrete variable into two children (`floor`/`ceil` bounds).
pub fn solve_mip(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    root_solver: RootSolver,
) -> SolveResult {
    let has_discrete = variables.iter().any(|v| v.vtype != VarType::Continuous);
    if !has_discrete {
        return solve_lp(variables, objective, constraints, root_solver);
    }

    let mut stack: Vec<Node> = vec![Node {
        overrides: HashMap::new(),
    }];
    let mut incumbent: Option<(f64, Vec<f64>)> = None;
    let mut node_count = 0usize;
    let mut node_limit_hit = false;

    while let Some(node) = stack.pop() {
        node_count += 1;
        if node_count > MAX_NODES {
            node_limit_hit = true;
            break;
        }

        let mut vars_eff = variables.to_vec();
        let mut bounds_infeasible = false;
        for (&j, &(lo, hi)) in node.overrides.iter() {
            let nl = vars_eff[j].lb.max(lo);
            let nh = vars_eff[j].ub.min(hi);
            if nl > nh + 1e-9 {
                bounds_infeasible = true;
                break;
            }
            vars_eff[j].lb = nl;
            vars_eff[j].ub = nh;
        }
        if bounds_infeasible {
            continue;
        }

        let relax = solve_lp(&vars_eff, objective, constraints, root_solver);
        if relax.status != Status::Optimal {
            continue;
        }
        let relax_obj = relax.objective.unwrap();
        if let Some((inc_obj, _)) = &incumbent {
            if !is_better(objective.sense, relax_obj, *inc_obj) {
                continue;
            }
        }

        let x = relax.x.unwrap();
        match most_fractional(variables, &x) {
            None => {
                let better = incumbent
                    .as_ref()
                    .map_or(true, |(inc, _)| is_better(objective.sense, relax_obj, *inc));
                if better {
                    // most_fractional only guarantees discrete components
                    // are within INT_TOL of an integer, not exactly equal
                    // to one, so round before recording the incumbent.
                    let mut x_rounded = x;
                    for (j, v) in variables.iter().enumerate() {
                        if v.vtype != VarType::Continuous {
                            x_rounded[j] = x_rounded[j].round();
                        }
                    }
                    let obj_rounded = objective.expr.constant
                        + objective
                            .expr
                            .coeffs
                            .iter()
                            .map(|(&j, &coeff)| coeff * x_rounded[j])
                            .sum::<f64>();
                    incumbent = Some((obj_rounded, x_rounded));
                }
            }
            Some(j) => {
                let val = x[j];
                let floor_v = val.floor();
                let ceil_v = val.ceil();

                // variable j's current bounds at this node: either an
                // override already in the map, or (first time j is
                // branched on along this path) its original bounds.
                let base = |m: &HashMap<usize, (f64, f64)>| {
                    *m.get(&j).unwrap_or(&(variables[j].lb, variables[j].ub))
                };

                let mut o_floor = node.overrides.clone();
                let (lo, hi) = base(&o_floor);
                o_floor.insert(j, (lo, hi.min(floor_v)));

                let mut o_ceil = node.overrides.clone();
                let (lo, hi) = base(&o_ceil);
                o_ceil.insert(j, (lo.max(ceil_v), hi));

                stack.push(Node { overrides: o_floor });
                stack.push(Node { overrides: o_ceil });
            }
        }
    }

    match incumbent {
        Some((obj, x)) => SolveResult {
            status: Status::Optimal,
            objective: Some(obj),
            x: Some(x),
            node_limit_hit,
        },
        // No node ever produced an all-integer relaxation. Re-solve the
        // unrestricted root relaxation once more just to classify *why*:
        // if it's infeasible/unbounded, so is the MIP (tightening bounds
        // can only shrink the feasible region further); if the root
        // relaxation is itself optimal yet no branch-and-bound node ever
        // reached an integer point, the MIP has no integer-feasible
        // solution even though its LP relaxation does.
        None => {
            let root = solve_lp(variables, objective, constraints, root_solver);
            match root.status {
                Status::Infeasible => root,
                Status::Unbounded => root,
                Status::Optimal => SolveResult {
                    status: Status::Infeasible,
                    objective: None,
                    x: None,
                    node_limit_hit,
                },
            }
        }
    }
}
