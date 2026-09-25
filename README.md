# ENOMOTO-Solver

[日本語版 README](README.ja.md)

ENOMOTO-Solver is a linear programming (LP) and mixed-integer linear programming (MIP) solver with a Python modelling interface and a Rust core.

Its LP engine implements the **slope–intercept dual two-phase method**, which treats the artificial-bound ("dual big-M") approach to dual phase 1 with a *symbolic* M:

- **Phase A (slope problem):** starting from the all-slack basis, which is dual feasible without any computation, solve the problem obtained by replacing infinite bounds with ±1 and the right-hand side with 0. Its optimal value decides whether the LP can have a finite optimum.
- **Phase B (intercept problem):** from the final basis of phase A, solve the problem with the slope held fixed by an ordinary dual simplex method.
- **Clean-up:** if variables remain on artificial bounds, move them to finite bounds with a primal ratio test, keeping optimality.

The method decides, **using the dual simplex method only and without choosing a numerical value for M**, whether an LP has a finite optimum, is unbounded, or is infeasible, and returns an optimal basic solution in the first case.

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

`solve()` returns a `Solution` with `status`, `objective` and `node_limit_hit`. The status is one of `"optimal"`, `"infeasible"`, `"unbounded"`, `"infeasible_or_unbounded"` or `"not_solved"`.

By default, the solver stops as soon as it proves that there is no finite optimum (at the end of phase A) and reports `"infeasible_or_unbounded"`. Pass `distinguish_infeasible_unbounded=True` to continue with phase B and tell the two cases apart:

```python
U = Model()
z = Variable(float, -math.inf, math.inf)
U.set_objective(z)                  # minimize z
U.add_constraint(z <= 5)

sol = U.solve(distinguish_infeasible_unbounded=True)
print(sol.status)                   # unbounded
```

### Integer variables

```python
K = Model()
items = [(Variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
K.set_objective(sum(v * value for v, value, _ in items), sense="maximize")
K.add_constraint(sum(v * weight for v, _, weight in items) <= 50)
sol = K.solve()
print(sol.objective, [round(v.value) for v, _, _ in items])   # 220.0 [0, 1, 1]
```

Models with integer variables are solved by a depth-first branch-and-bound method whose LP relaxations use the LP engine above. It is a simple implementation and is not intended to compete with dedicated MIP solvers.

## Benchmark against HiGHS (Netlib)

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
| `src/mip.rs` | Branch and bound |
| `src/params.rs` | Tolerances and tuning parameters |
| `docs/improvement_history.md` | Record of implementation changes and their measured effects |
| `benchmarks/netlib_dev_results.csv` | Netlib measurements taken during development (one row per problem and run; `source_file` names the original result file) |
| `benchmarks/mittelmann_results.*` | Mittelmann LPopt benchmark against HiGHS (2026-09-24, 44 public instances, 600 s limit; `scripts/run_mittelmann_benchmark.py`, Linux only) |

## Paper

The method and the numerical experiments are described in:

Kan Enomoto, *Complete classification of linear programs by the dual simplex method alone: the slope–intercept dual two-phase method* (in preparation).

## Use of generative AI

The ideas and theory of the method are the author's. Most of the implementation was carried out by an AI coding assistant (Claude, Anthropic) under the author's design and direction; the author decided which implementation techniques to adopt based on comparative experiments.

## Licence

[MIT License](LICENSE) © 2026 Kan Enomoto
