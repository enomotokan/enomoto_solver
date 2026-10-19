# ENOMOTO-Solver

[日本語版 README](https://github.com/enomotokan/enomoto_solver/blob/main/README.ja.md)

ENOMOTO-Solver is a solver for linear optimization problems (LP) and mixed-integer linear optimization problems (MIP), consisting of a Python modelling interface and a Rust core.

**Supported platforms:** CPython 3.9 or later (tested on 3.9, 3.13 and 3.14) on the following operating systems. The wheels use abi3 (the stable ABI of Python), so a single wheel per OS installs on every supported version.

| OS | Architecture |
|---|---|
| Windows | x86_64 |
| Linux (glibc 2.28 or later, manylinux_2_28) | x86_64, aarch64 |
| macOS | x86_64 (Intel), arm64 (Apple silicon) |

For LP, it implements the **slope–intercept dual two-phase method**. This is a method that treats the M of the artificial-bound method (the big-M method applied to the dual problem), one of the phase-1 methods of the dual simplex method, symbolically instead of fixing it as a number.

- **Phase A (slope problem):** starting from the all-slack basis, which is dual feasible without any computation, solve the problem obtained by replacing infinite bounds with ±1 and the right-hand side with 0. Its optimal value tells whether the original problem can have a finite optimal value.
- **Phase B (intercept problem):** from the final basis of phase A, solve the problem obtained by fixing the slope with the ordinary dual simplex method.
- **Post-processing:** if variables remain on artificial bounds, move them to finite bounds with a primal ratio test while keeping optimality.

In this way, **without choosing a numerical M and with the dual simplex method alone**, it decides whether an LP has a finite optimal value, is unbounded, or is infeasible, and returns an optimal basic solution when it has a finite optimal value.

The following are also implemented.

- **Proximal interior-point method + crossover:** a proximal interior-point method (IP-PMM, Mehrotra's predictor–corrector method) computes an interior optimal solution, and a crossover (primal and dual push phases and basis selection) turns it into an optimal basic solution. The finish is done by the simplex method.
- **Parallel execution of the two methods:** with `threads` set to 2 or more (the default is 4 when 4 or more CPUs are available, otherwise 1), an LP (for integer programs, a relaxation that cannot be warm-started) is solved at the same time by the slope–intercept dual two-phase method (1 thread) and the proximal interior-point method + crossover (the remaining threads), and the side that reaches a conclusion first is adopted (the other is stopped).
- **Mixed-integer linear optimization (MIP):** solved by branch and cut (constraint propagation, cutting planes, primal heuristics, symmetry handling, etc.). All relaxations are solved by the slope–intercept dual two-phase method (warm-started from the parent's optimal basis at nodes), and when the root relaxation has no finite optimal value, whether the problem is infeasible or unbounded is decided with the problem in which the variables of the intercept problem are restricted to integers (when `distinguish_infeasible_unbounded=True`). Giving the solver several threads (`threads` set to 2 or more) parallelizes branch and bound: after the root is processed, the threads share the unprocessed nodes and divide the tree search among themselves (the root relaxation is solved at the same time by the two-phase method and the proximal interior-point method + crossover).

## Installation

Install from PyPI (on the supported platforms above, a prebuilt wheel is installed, so Rust and other build tools are not needed).

```sh
pip install enomoto-solver
```

### Building from source

For development or for running the benchmarks, clone the repository and build it. The following are needed.

- Python 3.9 or later
- A Rust toolchain (`rustc`, `cargo`)
- [maturin](https://www.maturin.rs/) 1.5 or later (used to build the extension module)
- Only for running the benchmarks: [highspy](https://pypi.org/project/highspy/) and a C compiler (used to decompress the Netlib data)

```sh
git clone https://github.com/enomotokan/enomoto_solver.git
cd enomoto_solver
python -m venv .venv
# Windows: .venv\Scripts\activate   /   Linux, macOS: source .venv/bin/activate
pip install maturin
maturin develop --release
```

Always build with `--release`. A debug build is many times slower.

## Usage

### Defining and solving a problem

```python
import math
from enomoto_solver import Model, Variable

M = Model()
x = Variable(float, 0, 10)          # continuous variable 0 <= x <= 10
y = Variable(float, 0, math.inf)    # bounds may be infinite
M.set_objective(x + 2 * y, sense="maximize")
M.add_constraint(x + y <= 10)
M.add_constraint(x - y >= -4)

sol = M.solve()
print(sol.status, sol.objective)    # optimal 17.0
print(x.value, y.value)             # 3.0 7.0
```

- `Variable(type, lower bound, upper bound)` creates a variable in the most recently created `Model` (it can also be given explicitly with `model=...`). The type is `float` (continuous) or `int` (integer). Bounds may be `±math.inf` (integer variables included). A free variable is `Variable(float, -math.inf, math.inf)`.
- Linear expressions are written with `+`, `-`, scalar multiplication and division by numbers, and constraints with `<=`, `>=` and `==`. The objective and constraints are limited to linear expressions. Building an expression of degree two or higher, such as a product of variables (`x * y`), a power (`x ** 2`) or division by an expression (`1 / x`), raises a `TypeError` stating that it is not linear.
- `set_objective(f, sense="minimize" | "maximize")` sets the objective function (the default is minimization).
- Arguments of `solve()` (all optional):

  | Argument | Meaning |
  |---|---|
  | `threads` | Number of threads to use (default: 4 when 4 or more CPUs are available, otherwise 1; see `enomoto_solver.default_threads()`). With 2 or more, an LP is solved at the same time by the two-phase method and the interior-point method + crossover, and for integer programs the tree search is also shared. For details, see "Number of threads (`threads`)" below. |
  | `root_solver` | The LP method (default `"auto"`; see "Choosing the LP method" below). |
  | `distinguish_infeasible_unbounded` | Whether to distinguish infeasible from unbounded when there is no finite optimal value (default `False`; see "Result status, and infeasible or unbounded problems" below). |
  | `time_limit`, `mip_rel_gap`, `node_limit` | Termination criteria for integer programs (seconds, relative gap (default 0), number of nodes). |
  | `log` | Whether to output the log (default `True`; see "Log" below). |

- `solve()` always returns a `Solution` and raises no exception even when the problem is infeasible, unbounded or not solved. Judge the result by `Solution.status`. Only when `status` is `"optimal"` (for integer programs stopped at a limit, when there is an incumbent) does `objective` hold a value and can each variable's value be read with `.value`. Otherwise `objective` is `None` and reading `.value` raises `RuntimeError`.

### Result status, and infeasible or unbounded problems

`solve()` returns a `Solution` that has the following attributes. `status` is one of `"optimal"`, `"infeasible"`, `"unbounded"`, `"infeasible_or_unbounded"` and `"not_solved"` (for models with integer variables, there are also `"time_limit"` and `"node_limit"`).

| Attribute | Meaning |
|---|---|
| `status` | The result status (one of the above). |
| `objective` | The objective value of the solution (in the original sense, including the constant term). It holds a value when the status is `"optimal"`, and for integer programs stopped at a limit when there is an incumbent. Otherwise `None`. |
| `node_limit_hit` | Whether an integer program was stopped by the limit on the number of nodes (`node_limit`). Always `False` for LP. |
| `best_bound` | For integer programs, the proven bound on the optimal value (the lower bound for minimization, the upper bound for maximization). The optimal value lies between this value and `objective`. `inf` (`-inf` for maximization) if infeasible. `None` for LP. |
| `mip_gap` | For integer programs, the relative gap \|`objective` − `best_bound`\| / max(\|`objective`\|, 1). The solve ends with `"optimal"` when it becomes `mip_rel_gap` or less. `inf` if there is no incumbent. `None` for LP. |
| `nodes` | For integer programs, the number of branch-and-bound nodes processed (the total over all threads). `None` for LP. |

By default, the solver stops when it finds that there is no finite optimal value (at the end of phase A) and returns `"infeasible_or_unbounded"`. Passing `distinguish_infeasible_unbounded=True` makes it proceed to phase B and distinguish the two.

### Linear optimization (LP)

```python
M = Model()
z = Variable(float, -math.inf, math.inf)
M.set_objective(z)                  # minimize z
M.add_constraint(z <= 5)

sol = M.solve(distinguish_infeasible_unbounded=True)
print(sol.status)                   # unbounded
```

The same applies to models with integer variables. When phase A shows that the root relaxation has no finite optimal value, `"infeasible_or_unbounded"` is returned by default. With `distinguish_infeasible_unbounded=True`, the problem in which the variables of the intercept problem (the phase B problem) are restricted to integers is solved by branch and bound, and `"unbounded"` is returned if it has a feasible solution, `"infeasible"` if it is infeasible (because from a feasible solution x⁰ of the intercept problem and the slope x¹ of phase A, x⁰ + M x¹ becomes an integer solution that lowers the objective value without bound).

### Choosing the LP method

`solve(root_solver=...)` selects the LP method (it applies only to models without integer variables; the relaxations of integer programs are always solved by the slope–intercept dual two-phase method with warm starts; specifying anything other than `"auto"` for a model with integer variables issues a `UserWarning` and the setting is ignored).

| `root_solver` | Method |
|---|---|
| `"auto"` (default; `None` is the same) | If `threads` is 2 or more, run the slope–intercept dual two-phase method (1 thread) and the proximal interior-point method + crossover (the remaining threads) in parallel and adopt the side that reaches a conclusion first. With `threads=1`, solve with the two-phase method only. |
| `"simplex"` | The slope–intercept dual two-phase method only. |
| `"ipm_crossover"` | The proximal interior-point method + crossover only. If the interior-point method does not converge, the problem is solved again by the two-phase method. |
| `"interior"` | A proximal interior-point method (IP-PMM) with its own independent presolve. It does not give a basic solution and is intended for verification. |

Whatever the method, infeasibility and unboundedness are decided by the two-phase method. With `"auto"`, the interior-point method + crossover side is adopted only when it yields a basic solution whose optimality has been confirmed by the simplex method.

### Mixed-integer linear optimization (MIP)

```python
M = Model()
items = [(Variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
M.set_objective(sum(v * value for v, value, _ in items), sense="maximize")
M.add_constraint(sum(v * weight for v, _, weight in items) <= 50)
sol = M.solve(time_limit=60)
print(sol.objective, [round(v.value) for v, _, _ in items])   # 220.0 [0, 1, 1]
print(sol.best_bound, sol.mip_gap, sol.nodes)                 # 220.0 0.0 0
```

Models with integer variables are solved by branch and cut, which solves the relaxations with the slope–intercept dual two-phase method.

- **Presolve and constraint propagation:** merging of parallel rows, probing, activity-based bound tightening, conflict analysis and dual proofs.
- **Cutting planes:** CMIR (including tableau rows equivalent to Gomory cuts), lifted flow cover, {0, 1/2}-Chvátal–Gomory (zerohalf).
- **Primal heuristics:** rounding, fix-and-propagate, Feasibility Pump, Feasibility Jump, diving, rounding from the interior-point solution, improvement by sub-MIPs (RINS, RENS, DINS, Crossover, etc.).
- **Search:** reliability branching by pseudocosts, plunging (diving into child nodes with warm starts), restarts, symmetry detection and symmetry-breaking inequalities (including orbitopes).

#### Parallel search

When `threads` is k ≥ 2, the search runs in parallel as follows.

- **Relaxations that cannot be warm-started:** the root LP and the root LP after a restart have no basis to start from, so they are solved at the same time by the two-phase method (1 thread) and the proximal interior-point method + crossover (k − 1 threads). Node relaxations are solved by the two-phase method warm-started from the parent's optimal basis (they finish in a few iterations, so running the interior-point method at the same time has no chance of winning and only adds work). Sub-MIPs solved inside heuristics (RINS, etc.) are often small, so even their solves without a basis use the two-phase method only. After the tree search starts using k threads, no parallel execution is done.
- **Root:** the root cutting planes and heuristics are performed by the thread that solved the root (the main thread).
- **Tree search:** when the main thread has two unprocessed nodes, the remaining k − 1 threads are started from a copy of the tree after the root processing (for problems solved at the root, no additional threads are created). Each thread processes the nodes in its own queue. When a thread's queue becomes empty, the other threads hand it nodes with small lower bounds through a shared queue. The search ends when all threads are waiting and the shared queue is also empty.
- **Heuristic thread:** with k ≥ 4, ⌊k/4⌋ of the threads may leave the tree search and run improvement heuristics (adaptive large neighborhood search) continuously. A thread takes this role only while the large neighborhood search is estimated to close at least as much of the tree per second as one tree-search thread does (the tree progress is measured by the weight 2^−depth of the open nodes, and the heuristic's contribution by an exponential moving average of the tree weight its improvements prune). It returns to the tree search when this no longer holds or after 20 calls in a row without improving the incumbent. Set the environment variable `ENOMOTO_MIP_FOCUS=primal` to favor good incumbents over the optimality proof, or `ENOMOTO_MIP_NO_HEUR_ROLE=1` to disable this.
- **Information shared between threads:** incumbents (the best objective value is used for pruning in all threads), global bound tightenings, pseudocost observations (merged every 16 nodes), and dual proofs and conflicts (imported every 64 nodes).
- Since the order in which nodes are processed changes with the progress of the threads, with k ≥ 2 the number of nodes and the solve time may vary from run to run (the optimal value obtained does not change).

Termination criteria are specified with `time_limit` (seconds), `mip_rel_gap` (relative gap, default 0) and `node_limit` (number of nodes). When the solver stops at a limit, `status` becomes `"time_limit"` or `"node_limit"`, and if there is an incumbent, `objective` and each variable's `.value` can be read. The results are also stored in `best_bound` (the proven bound), `mip_gap` (relative gap) and `nodes` (number of nodes processed) of `Solution`.

The relative gap measures the difference between the objective value of the incumbent (the feasible solution with the best objective value found during the search) and the proven bound as

  relative gap = |objective value of the incumbent − proven bound| / max(|objective value of the incumbent|, 1)

(the denominator is set to 1 when it is less than 1, so that the ratio does not become extremely large when the objective value is close to 0). The optimal value lies between the proven bound and the objective value of the incumbent. During the search, nodes whose bound reaches "objective value of the incumbent − max(`mip_rel_gap` × max(|objective value of the incumbent|, 1), 10⁻⁹ × max(|objective value of the incumbent|, 1))" are pruned (when every objective coefficient of the integer variables is a multiple of a common step and there are no continuous variables in the objective, nodes that cannot improve by one step are pruned instead), so when no unprocessed nodes remain, the relative gap is `mip_rel_gap` or less (with the default `mip_rel_gap=0`, 10⁻⁹ or less). The solve then ends with `"optimal"`. With the default, `"optimal"` therefore means that the solution has been proven optimal up to the tolerances of the LP relaxations; specify e.g. `mip_rel_gap=1e-4` to stop earlier with a looser guarantee.

The solution returned is polished before it is reported: integer variables are rounded to exact integers, and if there are continuous variables, the LP of the original problem with the integer variables fixed to those values is solved again and its solution is taken when it violates the constraints no more than the original one and its objective value is not worse. `objective` is then recomputed from the returned values with compensated summation, so it agrees with the objective of the returned solution up to rounding error (on the MIPLIB 2017 instances in `benchmarks/`, the relative error against the reference optimal values is 10⁻¹⁵ or less).

### Number of threads (`threads`)

`solve(threads=...)` specifies the number of threads to use. If omitted, 4 threads are used when 4 or more CPUs are available to this process, and 1 otherwise (`enomoto_solver.default_threads()`). A value above the number of CPUs available to this process (`enomoto_solver.max_threads()`) is capped there. The whole solve, including parallel sections such as presolve, runs on this number of threads.

| `threads` | LP (`root_solver="auto"`) | MIP |
|---|---|---|
| 1 | Solved by the slope–intercept dual two-phase method only. | Both the relaxations and the tree search are solved on 1 thread. |
| k ≥ 2 | Solved at the same time by the two-phase method (1 thread) and the proximal interior-point method + crossover (k − 1 threads), adopting the side that reaches a conclusion first. | Relaxations that cannot be warm-started (the root, etc.) are solved at the same time in the same way as on the left, and after the root is processed the tree is shared among k threads for the search ("Parallel search" above). |

In parallel execution, the threads are divided before starting into separate pools for the two-phase method (1 thread) and for the interior-point method + crossover (the rest), so the two methods never take each other's threads. If the two-phase method finishes first, its basic solution is used as it is; if the interior-point method + crossover finishes first, the optimal basis obtained in the crossover's finish (the main loop of the two-phase method) is used as it is. The losing side is stopped, and its termination is not waited for.

```python
import enomoto_solver
sol = M.solve(threads=enomoto_solver.max_threads())   # use all available CPUs
```

### Log

By default, `solve()` outputs to standard output first a notice of the author, the license and the contact, together with the size of the problem, then the key points during the solve, and finally the result. The key points during the solve are as follows.

- LP: the size after presolve, the method used, and in parallel execution which method reached a conclusion first.
- MIP: the size after presolve, the value of the root LP, the change of the value by the root cutting planes, restarts, and the progress of the search (elapsed time, number of nodes processed, number of unprocessed nodes, proven bound, objective value of the incumbent and relative gap; every 5 seconds).

For the example in "Mixed-integer linear optimization (MIP)" above, the output is as follows.

```text
ENOMOTO-Solver 0.1.0 (MIT License)
Author: Kan Enomoto (https://enomoto-math-labo.com/en)
For optimization consulting requests and bug reports, contact enomotokan@gmail.com

Problem: 3 variables (3 integer), 1 constraints, 3 nonzeros, maximize; threads 1
Presolve: 1 rows, 3 columns -> 1 rows, 3 columns (0.00s)
Root LP: objective 2.2000000000e2 (0.00s)
     time      nodes      open        best bound         incumbent       gap
     0.0s          0         0     2.200000000e2     2.200000000e2     0.00%
Result: optimal, objective 220, best bound 220, gap 0.00%, nodes 0 (0.00s)
```

The columns of the search progress mean the following.

| Column | Meaning |
|---|---|
| `time` | Elapsed time since the start of the solve. |
| `nodes` | Number of nodes processed (whose relaxations have been solved), the total over all threads. |
| `open` | Number of unprocessed nodes: subproblems created by branching that are still in the queue and whose relaxations have not been solved yet (the total over all threads). The search ends when it becomes 0. |
| `best bound` | The proven bound (the lower bound on the optimal value for minimization, the upper bound for maximization). |
| `incumbent` | The objective value of the incumbent (the feasible solution with the best objective value found). `-` if there is none yet. |
| `gap` | The relative gap ("Mixed-integer linear optimization (MIP)" above) as a percentage. |

With `solve(log=False)`, neither the notice nor the log is output.

## Benchmark results

We compared with HiGHS 1.15.1, CLP 1.17.11 and SoPlex 8.1.0 on 207 LPs (measured on 29–30 September 2026). Each entry is "number of problems solved / shifted geometric mean of the solve time (seconds)" (shift 10 s; problems not solved are counted at the time limit of 600 s). Bold marks the best value in each row.

| Problem set | ENOMOTO | HiGHS | CLP | SoPlex |
|---|---|---|---|---|
| Netlib, with a finite optimal solution (93 problems) | 93 / **0.152** | 93 / 0.166 | 93 / 0.164 | 92 / 0.741 |
| Kennington (16 problems) | 16 / **0.964** | 16 / 1.68 | 16 / 1.16 | 16 / 5.76 |
| Mittelmann LPopt (40 problems) | 12 / **377** | 11 / 405 | **18** / 393 | 7 / 503 |
| Netlib infeasible (29 problems) | 29 / **0.00597** | 29 / 0.0154 | 29 / 0.0743 | 28 / 1.53 |
| Duals of the infeasible problems, unbounded (29 problems) | 29 / 0.0588 | 29 / 0.335 | 28 / 2.01 | 29 / **0.0332** |
| All (207 problems) | 179 / **10.6** | 178 / 11.0 | **184** / 11.3 | 172 / 13.4 |

The geometric mean over all 207 problems is 0.084 s for ENOMOTO, 0.183 s for HiGHS, 0.100 s for CLP and 0.146 s for SoPlex. ENOMOTO did not return a single wrong answer. On the 27 unbounded problems for which phase A detected that there is no finite optimal solution, the additional time needed to distinguish unbounded from infeasible was about 2% of the solve time.

- Machine: a laptop with an AMD Ryzen 7 5700U (8 cores 16 threads, 16 GB memory), Ubuntu 22.04 on WSL2 (memory limit 12 GB). ENOMOTO and HiGHS use 16 threads; CLP and SoPlex are sequential. All settings other than the time limit and the number of threads are the defaults.
- Times cover the solve call only (reading the MPS file and building the model are not included; presolve and postsolve are included) and are the median of 3 runs (1 run if it took 300 s or more). Each run is solved in a new process.
- Mittelmann consists of the 40 problems obtained by removing the four largest problems that do not fit in this machine's memory (thk_48, L2CTA3D, dlr2, Dual2_5000) from the 44 public problems available from plato.asu.edu.
- The full tables, per-problem times and raw data are in [benchmarks/paper/summary.md](https://github.com/enomotokan/enomoto_solver/blob/main/benchmarks/paper/summary.md), `per_problem.csv` and `results.json`. For the reproduction procedure (Linux), see [scripts/paper_bench/README.md](https://github.com/enomotokan/enomoto_solver/blob/main/scripts/paper_bench/README.md).

### Mixed-integer linear optimization (MIPLIB 2017)

We compared with HiGHS 1.15.1 on 40 small to medium problems from MIPLIB 2017 (measured on 12 October 2026). Both solvers were given 16 threads and a time limit of 60 s.

| | ENOMOTO | HiGHS |
|---|---|---|
| Problems solved / shifted geometric mean of the solve time (s) | **22** / **22.3** | 20 / 25.4 |

On the problems that both solvers proved optimal, no objective values disagreed. The per-problem results are below. "t" means the run was stopped by the time limit, and the objective is then the value of the best integer solution (incumbent) at that point ("-" means no integer solution). Bold marks the faster time, or, on problems where both solvers were stopped, the better objective.

| Problem | ENOMOTO [s] | HiGHS [s] | ENOMOTO objective | HiGHS objective |
|---|---:|---:|---:|---:|
| gen-ip002 | t | t | **-4783.73** | -4762.79 |
| gen-ip054 | t | t | **6852.68** | 6858.88 |
| pk1 | t | t | **11** | 14 |
| gr4x6 | **0.01** | 0.07 | 202.35 | 202.35 |
| markshare_4_0 | **3.46** | t | 1 | 1 |
| mas74 | t | t | **11801.2** | 12014.9 |
| mas76 | **39.42** | t | 40005.1 | 40005.1 |
| neos5 | t | t | 15 | 15 |
| p0201 | 2.28 | **0.92** | 7615 | 7615 |
| misc07 | **8.25** | 32.60 | 2810 | 2810 |
| flugpl | **0.03** | 0.11 | 1.2015e+06 | 1.2015e+06 |
| dcmulti | **1.10** | 1.37 | 188182 | 188182 |
| gt2 | 0.08 | **0.06** | 21166 | 21166 |
| khb05250 | 0.79 | **0.34** | 1.0694e+08 | 1.0694e+08 |
| qnet1 | 3.65 | **0.76** | 16029.7 | 16029.7 |
| rout | **9.76** | 38.98 | 1077.56 | 1077.56 |
| 10teams | 5.99 | **1.91** | 924 | 924 |
| air03 | **0.78** | 3.37 | 340160 | 340160 |
| blend2 | **0.56** | 3.37 | 7.59898 | 7.59898 |
| fiber | 1.70 | **1.09** | 405935 | 405935 |
| noswot | t | t | -41 | -41 |
| qiu | **36.18** | t | -132.873 | -132.873 |
| neos-911970 | t | **13.27** | 58.34 | 54.76 |
| 30n20b8 | t | t | - | **553** |
| binkar10_1 | 33.68 | **27.44** | 6742.2 | 6742.2 |
| neos-860300 | 34.44 | **20.09** | 3201 | 3201 |
| eil33-2 | **42.06** | t | 934.008 | 987.674 |
| nw04 | **8.11** | t | 16862 | 52674 |
| assign1-5-8 | t | t | 214 | **212** |
| beavma | 1.00 | **0.29** | 383285 | 383285 |
| blp-ar98 | t | t | **6995.88** | - |
| ran14x18-disj-8 | t | t | 3744 | 3744 |
| h80x6320d | t | **21.24** | 6757.85 | 6382.1 |
| supportcase26 | t | t | 1946.56 | **1772.96** |
| timtab1 | t | **55.36** | 880252 | 764774 |
| bppc4-08 | t | t | 62 | **53** |
| cvs16r128-89 | t | t | **-35** | -3 |
| mik-250-20-75-4 | **5.00** | 18.54 | -52301 | -52301 |
| neos-1456979 | t | t | - | **205** |
| graph20-20-1rand | t | t | -6 | **-8** |

- Machine: a laptop with an AMD Ryzen 7 5700U (8 cores 16 threads, 16 GB memory), Windows 11. All settings other than the time limit and the number of threads are the defaults (the default relative gap is 0 for ENOMOTO and 1e-4 for HiGHS).
- Times cover the solve call only (reading the MPS file and building the model are not included) and come from a single run. Each problem is solved in a separate subprocess; the shifted geometric mean uses a shift of 10 s and counts unsolved problems at 60 s.
- The raw data are in [benchmarks/miplib_results_16t_60s_20261012.json](https://github.com/enomotokan/enomoto_solver/blob/main/benchmarks/miplib_results_16t_60s_20261012.json). Reproduce with `python scripts/run_miplib_benchmark.py --time-limit 60 --threads 16` (the problems are downloaded from the MIPLIB site on the first run).

### Easy comparison with HiGHS on Netlib

The Netlib LP data has no license that explicitly permits redistribution, so it is not included in this repository ([docs/netlib-data.md](https://github.com/enomotokan/enomoto_solver/blob/main/docs/netlib-data.md)). Download and decompress it only once at the start.

```sh
python scripts/setup_netlib_data.py          # requires a network connection and a C compiler
```

Then compare with HiGHS (`pip install highspy`).

```sh
python -m enomoto_solver.benchmark_highs --max-vars 100000 --out netlib_results.csv
```

Each problem is solved in a separate subprocess. The measured time covers the solve call only (reading the MPS file and building the model are not included), and for both solvers it includes presolve and postsolve.

## Directory structure

| Path | Contents |
|---|---|
| `python/enomoto_solver/` | Python modelling interface (`Model`, `Variable`, `Function`, `Constraint`) and benchmark scripts |
| `src/model.rs` | Entry point called from Python |
| `src/presolve.rs`, `src/presolve/` | Presolve (scaling, row and column reductions, postsolve) |
| `src/simplex.rs` | Construction of the standard form, decomposition into connected components, primal simplex method |
| `src/simplex/slope_intercept_dual.rs` | Slope–intercept dual two-phase method |
| `src/simplex/lu.rs` | Sparse LU factorization and Forrest–Tomlin update |
| `src/simplex/crossover.rs` | Proximal interior-point method + crossover |
| `src/simplex/race.rs` | Parallel execution of the two-phase method and the interior-point method + crossover |
| `src/interior_point.rs`, `src/interior_point/` | Proximal interior-point method (IP-PMM) and factorization of the KKT system |
| `src/solver.rs` | Dispatch of the LP method (`root_solver`) |
| `src/mip/` | Branch and cut (constraint propagation, cutting planes, heuristics, symmetry) |
| `src/params.rs` | Tolerances and tuning parameters |
| `docs/improvement_history.md` | Record of implementation changes and their measured results |
| `benchmarks/netlib_dev_results.csv` | Netlib measurements taken during development (one row is one problem and one measurement; the `source_file` column is the name of the original result file) |
| `benchmarks/paper/` | Comparison with HiGHS, CLP and SoPlex included in the paper (207 problems, 600 s limit; `scripts/paper_bench/`) |
| `benchmarks/mittelmann_results.*` | Comparison with HiGHS on an earlier Mittelmann LPopt benchmark (2026-09-24, 44 public problems, 600 s limit; `scripts/run_mittelmann_benchmark.py`, Linux only) |

## Paper

The method and the numerical experiments are summarized in the following paper.

Kan Enomoto, "Complete classification of linear programming problems by the dual simplex method alone: the slope–intercept dual two-phase method" (in preparation).

## Use of generative AI

The idea and theory of the slope–intercept dual two-phase method for linear optimization and mixed-integer optimization are the author's. Most of the implementation was done by an AI coding assistant (Claude by Anthropic) under the author's design and direction, and the author decided which implementation techniques to adopt based on comparative experiments.

## License

[MIT License](https://github.com/enomotokan/enomoto_solver/blob/main/LICENSE) © 2026 Kan Enomoto
