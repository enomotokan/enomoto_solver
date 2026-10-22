# HiGHS で解を求めて元の列の順に 1 行 1 値で書き出す: highs_sol.py <mps> <秒> <出力>
import sys, highspy
h = highspy.Highs(); h.setOptionValue('output_flag', False)
h.setOptionValue('time_limit', float(sys.argv[2])); h.setOptionValue('threads', 4)
h.readModel(sys.argv[1]); h.run()
x = h.getSolution().col_value
print('obj', h.getInfo().objective_function_value, 'bound', h.getInfo().mip_dual_bound, h.modelStatusToString(h.getModelStatus()))
open(sys.argv[3], 'w').write('\n'.join(repr(v) for v in x) + '\n')
