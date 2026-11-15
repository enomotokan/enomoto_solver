import random, sys
from enomoto_solver import Model, Variable
g = random.Random(5); n = 60
M = Model(); xs = [Variable(int, 0, 3) for _ in range(n)]
w = [g.randint(20, 99) + 0.37 for _ in range(n)]; v = [g.randint(20, 99) for _ in range(n)]
for k in range(4):
    M.add_constraint(sum(g.randint(20, 99) * x for x in xs) <= 2500 + 37 * k)
M.add_constraint(sum(wj * x for wj, x in zip(w, xs)) <= 3001)
M.set_objective(sum((vj + 0.5 * (j % 2)) * x for j, (vj, x) in enumerate(zip(v, xs))) + 0.3, sense=sys.argv[1]) if "sense" in M.set_objective.__code__.co_varnames else None
s = M.solve(node_limit=int(sys.argv[2]), threads=1, log=False)
print(s.status, s.objective, s.best_bound)
