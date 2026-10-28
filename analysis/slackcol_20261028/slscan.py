import sys, os, subprocess, re
from concurrent.futures import ThreadPoolExecutor
CACHE='/home/user/enomoto_solver/.miplib_cache'
def one(args):
    n, flag = args
    env = dict(os.environ, ENOMOTO_MIP_LOG='1', ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3=f'/tmp/claude-0/-home-user-enomoto-solver/2059175a-4d57-5894-b463-ea424b7aad6b/scratchpad/pc_{n}_{flag}.mps', ENOMOTO_MIP_DEBUG_STOP_PRESOLVED='1')
    if flag: env['ENOMOTO_PRESOLVE_SLACK_COL'] = '1'; env['ENOMOTO_DEBUG_SLACK_COL'] = '1'
    p = subprocess.run([sys.executable, 'scripts/mip_bench/bench.py', '--worker', f'{CACHE}/{n}.mps', '--tl', '60', '--threads', '1'], capture_output=True, text=True, env=env, timeout=600)
    m = [(sum(int(v) for v in re.findall(r'DEBUG_SLACK_COL: round=\d+ columns=(\d+)', p.stderr)), re.findall(r'free columns \d+ -> (\d+)', p.stderr)[:1])]
    os.remove(env['ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3']) if os.path.exists(env['ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3']) else None
    return n, flag, m[0] if m else None
probs = sys.argv[1:]
with ThreadPoolExecutor(4) as ex:
    res = list(ex.map(one, [(n, f) for n in probs for f in (0, 1)]))
d = {}
for n, f, m in res: d.setdefault(n, {})[f] = m
for n in probs:
    a, b = d[n].get(0), d[n].get(1)
    if a != b: print(f"{n:22s} default {a}  SLACK {b}")
print('done', len(probs))
