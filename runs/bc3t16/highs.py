"""HiGHS 1.15.1 で beasleyC3 (TH スレッド、既定 16、mip_rel_gap=0)。使い方: python highs.py <回> <秒>"""
import sys, highspy
from pathlib import Path
REPO = Path(__file__).resolve().parents[2]
rep, T = sys.argv[1], float(sys.argv[2])
import os; TH = int(os.environ.get('TH', '16'))
h = highspy.Highs()
h.setOptionValue('threads', TH); h.setOptionValue('mip_rel_gap', 0.0); h.setOptionValue('output_flag', True)
h.setOptionValue('time_limit', T)
h.setOptionValue('log_file', str(REPO / 'runs/bc3t16/logs' / (f'highs_{rep}.log' if os.environ.get('PROB', 'beasleyC3') == 'beasleyC3' else f"{os.environ['PROB']}_highs_{rep}.log")))
prob = os.environ.get('PROB', 'beasleyC3'); h.readModel(str(REPO / '.miplib_cache' / (prob + '.mps.gz')))
h.run()
print('HIGHS', rep, h.getModelStatus(), h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count, h.getRunTime())
