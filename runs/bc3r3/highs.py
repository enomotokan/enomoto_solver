"""HiGHS 1.15.1 で beasleyC3 (4 スレッド、mip_rel_gap=0)。使い方: python highs.py <回> <秒>"""
import sys, highspy
from pathlib import Path
REPO = Path(__file__).resolve().parents[2]
rep, T = sys.argv[1], float(sys.argv[2])
h = highspy.Highs()
h.setOptionValue('threads', 4); h.setOptionValue('mip_rel_gap', 0.0); h.setOptionValue('output_flag', True)
h.setOptionValue('time_limit', T)
h.setOptionValue('log_file', str(REPO / 'runs/bc3r3/logs' / f'highs_{rep}.log'))
h.readModel(str(REPO / '.miplib_cache/beasleyC3.mps.gz'))
h.run()
print('HIGHS', rep, h.getModelStatus(), h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count, h.getRunTime())
