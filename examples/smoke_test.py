import math

from enomoto_solver import Constraint, Function, InfeasibleError, Model, Variable


def close(a, b, tol=1e-6):
    assert abs(a - b) < tol, f"{a} != {b}"


# 1) Simple LP (maximize)
M = Model()
x = Variable(float, 0, 10)
y = Variable(float, 0, 10)
f = x + 2 * y
assert isinstance(f, Function)
M.set_objective(f, sense="maximize")
g1 = x + y <= 10
g2 = 0 <= x
g3 = Function.__ge__(y, 0)
assert isinstance(g1, Constraint) and isinstance(g2, Constraint)
M.add_constraint(g1)
M.add_constraint(g2)
M.add_constraint(y >= 0)
sol = M.solve()
print("LP1:", sol, "x=", x.value, "y=", y.value)
close(sol.objective, 20.0)
close(y.value, 10.0)
close(x.value, 0.0)

# 2) Minimize with >= and == constraints
M2 = Model()
a = Variable(float, 0, 1000)
b = Variable(float, 0, 1000)
M2.set_objective(a + b)  # default sense=minimize
M2.add_constraint(a + 2 * b >= 6)
M2.add_constraint(a - b == 0)
sol2 = M2.solve()
print("LP2:", sol2, "a=", a.value, "b=", b.value)
close(sol2.objective, 4.0)
close(a.value, 2.0)
close(b.value, 2.0)

# 3) Wide (but finite) bounds + negative bound handling
M3 = Model()
z = Variable(float, -1_000_000, 1_000_000)
M3.set_objective(z)
M3.add_constraint(z >= -5)
M3.add_constraint(z <= 100)
sol3 = M3.solve()
close(sol3.objective, -5.0)
print("LP3 (wide bounds):", sol3, "z=", z.value)

# 4) Small MIP (knapsack-ish, binary variables)
M4 = Model()
items = [(Variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
obj = sum(v * val for v, val, _ in items)
M4.set_objective(obj, sense="maximize")
M4.add_constraint(sum(v * w for v, _, w in items) <= 50)
sol4 = M4.solve()
print("MIP knapsack:", sol4, [round(v.value) for v, _, _ in items])
close(sol4.objective, 220.0)

# 5) Integer variables
M5 = Model()
p = Variable(int, 0, 10)
q = Variable(int, 0, 10)
M5.set_objective(3 * p + 5 * q, sense="maximize")
M5.add_constraint(p + q <= 7)
M5.add_constraint(2 * p + q <= 10)
sol5 = M5.solve()
print("MIP2:", sol5, "p=", p.value, "q=", q.value)

# 6) Infeasible model
M6 = Model()
w = Variable(float, 0, 5)
M6.set_objective(w)
M6.add_constraint(w >= 10)
try:
    M6.solve()
    raise AssertionError("expected InfeasibleError")
except InfeasibleError:
    print("Infeasible correctly detected")

# 7) Infinite bounds are accepted (model.rs::add_variable's own docs: no
# longer rejected at the PyO3 boundary — only NaN and lb > ub are).
M7 = Model()
v7 = Variable(float, 0, math.inf, model=M7)
assert v7.lb == 0 and v7.ub == math.inf
print("Infinite upper bound correctly accepted")

# 8) Type errors
M8 = Model()
try:
    M8.set_objective(123)
    raise AssertionError("expected TypeError")
except TypeError:
    print("set_objective TypeError correctly raised")

try:
    M8.add_constraint(123)
    raise AssertionError("expected TypeError")
except TypeError:
    print("add_constraint TypeError correctly raised")

print("ALL OK")
