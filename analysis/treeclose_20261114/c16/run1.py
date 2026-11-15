import sys, os, json, subprocess, resource, importlib.util, time
from pathlib import Path
W = Path('/home/user/enomoto_solver')
sp = importlib.util.spec_from_file_location('c', W / 'analysis/cand1116_20261115/cfg.py'); m = importlib.util.module_from_spec(sp); sp.loader.exec_module(m)
name, seed, tl, extra = sys.argv[1], sys.argv[2], sys.argv[3], json.loads(sys.argv[4]); prob = sys.argv[5] if len(sys.argv) > 5 else 'fiball'; sys.path.insert(0, str(W / 'scripts')); import run_miplib_benchmark as rb; mps = rb.fetch(prob, W / '.miplib_cache')
cfg = dict(getattr(m, name.split('.')[0])); 
for k, v in extra.items():
    if v is None: cfg.pop(k, None)
    else: cfg[k] = v
env = {k: v for k, v in os.environ.items() if not k.startswith('ENOMOTO_')}
env.update(cfg, ENOMOTO_MIP_TCTRACE='1', ENOMOTO_MIP_THREADS='4', ENOMOTO_T_MIP_SEED=seed, ENOMOTO_MIP_LOG='1')
t = time.time()
with open(f'/tmp/claude-0/rdump/c16/{name}.log', 'w') as lf:
    r = subprocess.run([sys.executable, str(W / 'scripts/run_miplib_benchmark.py'), '--worker-ours', str(mps), '--time-limit', tl, '--threads', '4'], env=env, cwd=W, stdout=subprocess.PIPE, stderr=lf, text=True)
res = [l for l in r.stdout.splitlines() if l.startswith('{')][-1]
print(name, res[:200], 'rss_mb', resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss // 1024, 'wall', round(time.time() - t))
