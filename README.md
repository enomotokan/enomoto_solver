# ENOMOTO-Solver

[日本語版 README](README.ja.md)

ENOMOTO-Solver is a linear programming (LP) and mixed-integer linear programming (MIP) solver with a Python modelling interface and a Rust core.

Its LP engine implements the **slope–intercept dual two-phase method**, which treats the artificial-bound ("dual big-M") approach to dual phase 1 with a *symbolic* M:

- **Phase A (slope problem):** starting from the all-slack basis, which is dual feasible without any computation, solve the problem obtained by replacing infinite bounds with ±1 and the right-hand side with 0. Its optimal value decides whether the LP can have a finite optimum.
- **Phase B (intercept problem):** from the final basis of phase A, solve the problem with the slope held fixed by an ordinary dual simplex method.
- **Clean-up:** if variables remain on artificial bounds, move them to finite bounds with a primal ratio test, keeping optimality.

The method decides, **using the dual simplex method only and without choosing a numerical value for M**, whether an LP has a finite optimum, is unbounded, or is infeasible, and returns an optimal basic solution in the first case.

The solver also includes:

- **Proximal interior-point method + crossover:** a PIQP-style proximal interior-point method (IP-PMM with Mehrotra's predictor–corrector) computes an interior optimal solution, and a crossover following Liu & Lu (2024) (primal and dual push phases plus basis selection) turns it into an optimal basic solution, finished off by the simplex method.
- **Running both methods in parallel:** with `threads >= 2` (the default is 1), an LP (and every MIP relaxation that cannot be warm-started) is solved by the slope–intercept dual two-phase method on one thread and by the interior-point method + crossover on the remaining threads at the same time; the first to reach a conclusion wins and the other is stopped.
- **Mixed-integer linear programming (MIP):** branch and cut with propagation, cutting planes, primal heuristics and symmetry handling. Every relaxation is solved by the slope–intercept dual two-phase method (warm-started from the parent's optimal basis at nodes), and when the root relaxation has no finite optimum, the intercept problem with integrality restrictions decides whether the MIP is infeasible or unbounded (with `distinguish_infeasible_unbounded=True`). With `threads >= 2`, the root relaxation is raced against the interior-point method + crossover and the tree search is shared among several threads.

## Requirements

- Python 3.9 or later
- A Rust toolchain (`rustc`, `cargo`)
- [maturin](https://www.maturin.rs/) 1.5 or later (used to build the extension)
- For benchmarks only: [highspy](https://pypi.org/project/highspy/) and a C compiler (to decompress the Netlib data)

## Installation

```sh
git clone https://github.com/enomotokan/enomoto_solver.git
cd enomoto_solver
python -m venv .venv
# Windows: .venv\Scripts\activate   /   Linux, macOS: source .venv/bin/activate
pip install maturin
maturin develop --release
```

Always build with `--release`; a debug build is many times slower.

## Usage

### Defining and solving a model

```python
import math
from enomoto_solver import Model, Variable

M = Model()
x = Variable(float, 0, 10)          # continuous variable, 0 <= x <= 10
y = Variable(float, 0, math.inf)    # bounds may be infinite
M.set_objective(x + 2 * y, sense="maximize")
M.add_constraint(x + y <= 10)
M.add_constraint(x - y >= -4)

sol = M.solve()
print(sol.status, sol.objective)    # optimal 17.0
print(x.value, y.value)             # 3.0 7.0
```

- `Variable(vtype, lb, ub)` creates a variable in the most recently created `Model` (or pass `model=...`). `vtype` is `float` (continuous) or `int` (integer). Bounds may be `±math.inf`; a free variable is `Variable(float, -math.inf, math.inf)`.
- Linear expressions are written with `+`, `-` and multiplication by scalars. Constraints are written with `<=`, `>=` and `==`.
- `set_objective(f, sense="minimize" | "maximize")` sets the objective (default: minimize).
- `solve()` always returns a `Solution`; it never raises because the model is infeasible, unbounded or not solved. Check `Solution.status`. Only when the status is `"optimal"` is `objective` set and each variable's value available as `.value`; otherwise `objective` is `None` and reading `.value` raises `RuntimeError`.

### Result status and infeasible or unbounded problems

`solve()` returns a `Solution` with `status`, `objective` and `node_limit_hit`. The status is one of `"optimal"`, `"infeasible"`, `"unbounded"`, `"infeasible_or_unbounded"` or `"not_solved"` (models with integer variables can also return `"time_limit"` or `"node_limit"`).

By default, the solver stops as soon as it proves that there is no finite optimum (at the end of phase A) and reports `"infeasible_or_unbounded"`. Pass `distinguish_infeasible_unbounded=True` to continue with phase B and tell the two cases apart:

```python
M = Model()
z = Variable(float, -math.inf, math.inf)
M.set_objective(z)                  # minimize z
M.add_constraint(z <= 5)

sol = M.solve(distinguish_infeasible_unbounded=True)
print(sol.status)                   # unbounded
```

The same holds for models with integer variables. When phase A shows that the root relaxation has no finite optimum, the default result is `"infeasible_or_unbounded"`. With `distinguish_infeasible_unbounded=True`, the intercept problem (the phase B problem) with its variables restricted to integers is solved by branch and bound: if it has a feasible solution the result is `"unbounded"`, otherwise `"infeasible"` (a feasible intercept solution x⁰ and the phase A slope x¹ give integer solutions x⁰ + M x¹ whose objective decreases without bound).

### Choosing the LP method

`solve(root_solver=...)` selects the LP method. It applies only to models without integer variables; MIP relaxations are always solved by the slope–intercept dual two-phase method with warm starts.

| `root_solver` | Method |
|---|---|
| `"auto"` (default; `None` is the same) | With `threads >= 2`, run the slope–intercept dual two-phase method (1 thread) and the interior-point method + crossover (the remaining threads) in parallel and take whichever concludes first; with `threads=1` (the default) use the two-phase method alone. |
| `"simplex"` | The slope–intercept dual two-phase method only. |
| `"ipm_crossover"` | The interior-point method + crossover only; falls back to the two-phase method if the interior-point method does not converge. |
| `"interior"` | A standalone proximal interior-point method (IP-PMM) with its own presolve. Returns a non-basic solution; intended for cross-checking. |

Whatever the method, infeasibility and unboundedness are decided by the two-phase method. With `"auto"`, the interior-point + crossover result is used only when it yields a basic solution whose optimality has been verified by the simplex method.

### Number of threads (`threads`)

`solve(threads=...)` sets the number of threads to use (default 1). Values above the number of CPUs available to the process (`enomoto_solver.max_threads()`, which the environment variable `ENOMOTO_MIP_MAX_THREADS` can lower further) are capped there. The whole solve, including parallel sections such as presolve, runs on this many threads.

| `threads` | LP (`root_solver="auto"`) | MIP |
|---|---|---|
| 1 (default) | Solved by the slope–intercept dual two-phase method alone. | Relaxations and the tree search use one thread. |
| k ≥ 2 | Solved by the two-phase method (1 thread) and the interior-point method + crossover (k − 1 threads) at the same time; whichever concludes first wins. | Relaxations that cannot be warm-started (such as the root) are raced in the same way, and after the root the tree search is shared among k threads (see "Parallel branch and bound" below). |

When the two methods race, the threads are split into separate pools before either starts — one thread for the two-phase method and the rest for the interior-point method + crossover — so the two never compete for each other's threads. If the two-phase method finishes first, its basic solution is used as is; if the interior-point method + crossover finishes first, the optimal basis from the crossover's clean-up (the two-phase method's main loop) is used as is. The loser is stopped and not waited for.

```python
import enomoto_solver
sol = M.solve(threads=enomoto_solver.max_threads())   # use every available CPU
```

### Integer variables

```python
M = Model()
items = [(Variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
M.set_objective(sum(v * value for v, value, _ in items), sense="maximize")
M.add_constraint(sum(v * weight for v, _, weight in items) <= 50)
sol = M.solve(time_limit=60)
print(sol.objective, [round(v.value) for v, _, _ in items])   # 220.0 [0, 1, 1]
print(sol.best_bound, sol.mip_gap, sol.nodes)                 # 220.0 0.0 0
```

Models with integer variables are solved by branch and cut, with LP relaxations solved by the slope–intercept dual two-phase method (a simplified implementation of techniques from HiGHS and SCIP):

- **Presolve and propagation:** merging parallel rows, probing, activity-based bound tightening, conflict analysis and dual proofs.
- **Cutting planes:** CMIR (including tableau rows, i.e. Gomory-like cuts), lifted flow covers, {0, 1/2}-Chvátal–Gomory (zero-half) cuts.
- **Primal heuristics:** rounding, fix-and-propagate, Feasibility Pump, Feasibility Jump, diving, rounding from the interior-point solution, and sub-MIP improvement (RINS, RENS, DINS, Crossover, etc.).
- **Search:** reliability branching on pseudocosts, plunging (diving into a child with a warm start), restarts, symmetry detection with symmetry-breaking inequalities (including orbitopes).

#### Parallel branch and bound

With `threads` = k ≥ 2, branch and bound runs in parallel as follows.

- **Relaxations without a starting basis:** the root LP and the root LP after a restart have no basis to start from, so they are solved by the two-phase method (1 thread) and the interior-point method + crossover (k − 1 threads) at the same time. Node relaxations are solved by the two-phase method warm-started from the parent's optimal basis (they finish in a few iterations, so racing the interior-point method would only add overhead). Sub-MIPs inside heuristics (RINS etc.) are usually small, so even their cold solves use the two-phase method alone. Once the tree search uses all k threads, no more races are started.
- **Root:** root cutting planes and heuristics are run by the thread that solved the root (the main thread).
- **Tree search:** once the main thread has two open nodes, the other k − 1 threads start from a copy of the tree after the root (a problem closed at the root never starts them). Each thread processes the nodes in its own queue; when a thread runs out of nodes, the others hand it nodes with small lower bounds through a shared queue. The search ends when every thread is waiting and the shared queue is empty.
- **Shared between threads:** incumbents (the best objective value is used for pruning by every thread), global bound tightenings, pseudocost observations (merged every 16 nodes), and dual proofs and conflicts (imported every 64 nodes).
- Because the order in which nodes are processed depends on how fast each thread progresses, node counts and solve times can vary from run to run with k ≥ 2 (the optimal value does not).

Limits are set with `time_limit` (seconds), `mip_rel_gap` (relative gap, default 1e-4) and `node_limit` (number of nodes). When a limit stops the search, `status` is `"time_limit"` or `"node_limit"`, and if an incumbent exists, `objective` and each variable's `.value` can be read. `Solution` also reports `best_bound` (the proven bound), `mip_gap` (relative gap) and `nodes` (nodes processed).

## Benchmark results

Comparison with HiGHS 1.15.1, CLP 1.17.11 and SoPlex 8.1.0 on 207 LPs (measured 2026-09-29/30). Each entry is *problems solved / shifted geometric mean of the solve time in seconds* (shift 10 s; unsolved problems counted at the 600 s time limit). Bold marks the best value in each row.

| Test set | ENOMOTO | HiGHS | CLP | SoPlex |
|---|---|---|---|---|
| Netlib, finite optimum (93) | 93 / **0.152** | 93 / 0.166 | 93 / 0.164 | 92 / 0.741 |
| Kennington (16) | 16 / **0.964** | 16 / 1.68 | 16 / 1.16 | 16 / 5.76 |
| Mittelmann LPopt (40) | 12 / **377** | 11 / 405 | **18** / 393 | 7 / 503 |
| Netlib infeasible (29) | 29 / **0.00597** | 29 / 0.0154 | 29 / 0.0743 | 28 / 1.53 |
| Duals of the infeasible problems, unbounded (29) | 29 / 0.0588 | 29 / 0.335 | 28 / 2.01 | 29 / **0.0332** |
| All (207) | 179 / **10.6** | 178 / 11.0 | **184** / 11.3 | 172 / 13.4 |

Over all 207 problems the plain geometric mean is 0.084 s for ENOMOTO, 0.183 s for HiGHS, 0.100 s for CLP and 0.146 s for SoPlex. ENOMOTO returned no wrong answer; on the 27 unbounded problems where phase A detected that no finite optimum exists, telling unboundedness from infeasibility added about 2% to the solve time.

- Machine: AMD Ryzen 7 5700U laptop (8 cores / 16 threads, 16 GB), WSL2 Ubuntu 22.04 with a 12 GB memory limit. ENOMOTO and HiGHS use 16 threads; CLP and SoPlex are serial. All solvers use default settings apart from the time limit and thread count.
- Solve time only (reading the MPS file and building the model are excluded; presolve and postsolve are included), median of 3 runs (1 run for runs of 300 s or more), each run in a fresh process.
- Mittelmann: the 44 public instances available from plato.asu.edu minus the four largest (thk_48, L2CTA3D, dlr2, Dual2_5000), which do not fit in this machine's memory.
- Full tables, per-problem times and raw data: [benchmarks/paper/summary.md](benchmarks/paper/summary.md), `per_problem.csv`, `results.json`. To reproduce (Linux): [scripts/paper_bench/README.md](scripts/paper_bench/README.md).

### Quick comparison with HiGHS on Netlib

The Netlib LP data is not included in this repository because it carries no explicit redistribution licence (see [docs/netlib-data.md](docs/netlib-data.md)). Download and decompress it once:

```sh
python scripts/setup_netlib_data.py          # needs network access and a C compiler
```

Then compare with HiGHS (`pip install highspy`):

```sh
python -m enomoto_solver.benchmark_highs --max-vars 100000 --out netlib_results.csv
```

Each problem is solved in its own subprocess. The reported times cover the solve call only (reading the MPS file and building the model are excluded) and include presolve and postsolve for both solvers.

## Repository layout

| Path | Contents |
|---|---|
| `python/enomoto_solver/` | Python modelling interface (`Model`, `Variable`, `Function`, `Constraint`) and the benchmark script |
| `src/model.rs` | Entry point called from Python |
| `src/presolve.rs`, `src/presolve/` | Presolve (scaling, row/column reductions, postsolve) |
| `src/simplex.rs` | Standard-form construction, component decomposition, primal simplex |
| `src/simplex/slope_intercept_dual.rs` | Slope–intercept dual two-phase method |
| `src/simplex/lu.rs` | Sparse LU factorization and Forrest–Tomlin update |
| `src/simplex/crossover.rs` | Proximal interior-point method + crossover |
| `src/simplex/race.rs` | Parallel run of the two-phase method and interior-point + crossover |
| `src/interior_point.rs`, `src/interior_point/` | Proximal interior-point method (IP-PMM) and KKT factorization |
| `src/solver.rs` | Dispatch to the LP method (`root_solver`) |
| `src/mip/` | Branch and cut (propagation, cutting planes, heuristics, symmetry) |
| `src/params.rs` | Tolerances and tuning parameters |
| `docs/improvement_history.md` | Record of implementation changes and their measured effects |
| `benchmarks/netlib_dev_results.csv` | Netlib measurements taken during development (one row per problem and run; `source_file` names the original result file) |
| `benchmarks/paper/` | Benchmark against HiGHS, CLP and SoPlex used in the paper (207 problems, 600 s limit; `scripts/paper_bench/`) |
| `benchmarks/mittelmann_results.*` | Earlier Mittelmann LPopt benchmark against HiGHS (2026-09-24, 44 public instances, 600 s limit; `scripts/run_mittelmann_benchmark.py`, Linux only) |

## Paper

The method and the numerical experiments are described in:

Kan Enomoto, *Complete classification of linear programs by the dual simplex method alone: the slope–intercept dual two-phase method* (in preparation).

## Use of generative AI

The ideas and theory of the method are the author's. Most of the implementation was carried out by an AI coding assistant (Claude, Anthropic) under the author's design and direction; the author decided which implementation techniques to adopt based on comparative experiments.

## Licence

[MIT License](LICENSE) © 2026 Kan Enomoto
