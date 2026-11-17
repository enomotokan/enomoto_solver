import sys, os, json, subprocess, importlib.util
from pathlib import Path
W = Path('/home/user/enomoto_solver')
sp = importlib.util.spec_from_file_location('c', W / 'analysis/cand1116_20261115/cfg.py'); m = importlib.util.module_from_spec(sp); sp.loader.exec_module(m)
tc, seed, nl = sys.argv[1], sys.argv[2], sys.argv[3]
env = {k: v for k, v in os.environ.items() if not k.startswith('ENOMOTO_')}
env.update(m.CAND1116, ENOMOTO_MIP_TC=tc, ENOMOTO_MIP_THREADS='1', ENOMOTO_T_MIP_SEED=seed, ENOMOTO_MIP_LOG='1', ENOMOTO_MIP_NODE_LIMIT=nl, ENOMOTO_MIP_TCTRACE='1', ENOMOTO_MIP_TICKS='ntwumc')
name = f'fbt_{tc}_s{seed}' + (sys.argv[4] if len(sys.argv) > 4 else '')
with open(f'/tmp/claude-0/rdump/c16/{name}.log', 'w') as lf:
    r = subprocess.run([sys.executable, str(W / 'scripts/run_miplib_benchmark.py'), '--worker-ours', str(W / '.miplib_cache/fiball.mps'), '--time-limit', '600', '--threads', '1'], env=env, cwd=W, stdout=subprocess.PIPE, stderr=lf, text=True)
print(name, [l for l in r.stdout.splitlines() if l.startswith('{')][-1][:160])
