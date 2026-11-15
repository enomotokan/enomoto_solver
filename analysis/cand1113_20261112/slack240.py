"""fuzz_slack seed 240 (束 cand1113 で状態が infeasible_or_unbounded になる例)。
ENOMOTO_MIP_HNFPRE=0 python slack240.py → infeasible_or_unbounded、付けなければ infeasible (HiGHS も Infeasible)。
等式 7.5x1 - 4.5x2 - 7.5x4 - 7.5x5 - x8 = 7 は左辺の整数部分が 1.5 の倍数、x8 ∈ {0,1} なので 7 にも 8 にも届かず整数解がない。
緩和は x2・x9・x11 を動かすと目的が下に非有界。"""
import os
os.environ.setdefault("ENOMOTO_MIP_THREADS", "1")
from enomoto_solver import Model, Variable
inf = float("inf")
M = Model()
x = [Variable(int, 0, 1), Variable(int, -5, inf), Variable(int, 2, inf), Variable(float, -2, 34),
     Variable(int, 0, 5), Variable(int, 0, 1), Variable(int, 0, 15), Variable(float, -5, 48),
     Variable(int, 0, 1), Variable(int, -3, inf), Variable(int, -1, 20), Variable(int, -1, inf)]
M.set_objective(-5*x[0] - 4*x[2] + 8*x[5] - 5*x[8] - 6*x[9] + 9*x[10] - 3*x[11], "minimize")
M.add_constraint(4.5*x[6] >= -5); M.add_constraint(4.5*x[6] <= 3)
M.add_constraint(-7.5*x[1] + x[3] - 2*x[5] - 3*x[7] <= -8)
M.add_constraint(7.5*x[4] + 7.5*x[5] + 2*x[6] + 7.5*x[10] >= -2); M.add_constraint(7.5*x[4] + 7.5*x[5] + 2*x[6] + 7.5*x[10] <= 1)
M.add_constraint(-1*x[10] >= 0); M.add_constraint(-1*x[10] <= 7)
M.add_constraint(7.5*x[1] - 4.5*x[2] - 7.5*x[4] - 7.5*x[5] - x[8] == 7)
M.add_constraint(-x[0] + 3*x[1] + 2*x[3] + 7.5*x[5] + 4.5*x[9] <= 6)
print(M.solve(mip_rel_gap=1e-9).status)
