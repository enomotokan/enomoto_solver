"""fuzz_objint.py の seed 14 を縮めた誤答の再現 (既定の動き、OBJINT とは無関係)。実行不能な問題 (2 行目 + 3 行目で 0 >= 2 + ...) を
ENOMOTO は「最適」と答える (返す点は列 32 の下限 1 を破る)。34 列のうち 17 列は行にも目的にも出ない列 (これを外すと正しく実行不能)。
python wrong_seed14.py"""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import fuzz_objint as f
inf = float("inf")
LP = ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, -4.0], [5.0, 1.0, 1.0, 1.0, 1.0, 1.0, 3.0, 1.0, 3.0, 3.0, 3.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 5.0, 1.0, 1.0, 5.0, 1.0, 1.0, 1.0, 1.0, 1.0, 5.0, 1.0, 1.0, 1.0, 1.0, 58.0, inf], [0.0, -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, True, False, False], [[(33, 1.0), (1, -6.0), (22, -4.5), (23, -9.0), (30, -5.5), (16, -2.5)], [(33, 1.0), (32, -1.0), (13, -6.0)], [(32, 1.0), (33, -1.0), (12, -7.5), (23, -7.0), (31, -6.0)], [(11, 9.0), (4, 1.0), (0, 7.0), (21, 8.0), (7, 5.0), (20, 8.0), (28, 6.0), (31, 2.0)]], [-0.5, 0.0, 2.0, -inf], [inf, inf, inf, 19.0], 0.0, 'minimize')
print("HiGHS", f.highs(LP))
s, o, x = f.ours(LP)
print("ENOMOTO", s, o, "feasible" if x and f.feasible(LP, x) else "point infeasible", x and (x[32], x[33]))
