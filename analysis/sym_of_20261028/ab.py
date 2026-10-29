"""設定ごとに問題を 1 問 1 サブプロセスで解き、jsonl に書き足す (再開可)。
python ab.py out.jsonl 時間 スレッド 並列 回数 設定名=ENV1=V1,ENV2=V2 ... -- 問題..."""
import sys, os, json, subprocess, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
REPO = Path('/home/user/enomoto_solver')
BENCH = REPO / 'scripts' / 'mip_bench' / 'bench.py'
CACHE = REPO / '.miplib_cache'
out = Path(sys.argv[1]); tl = float(sys.argv[2]); threads = int(sys.argv[3]); jobs = int(sys.argv[4]); reps = int(sys.argv[5])
k = sys.argv.index('--')
cfgs = {}
for c in sys.argv[6:k]:
    name, _, envs = c.partition('=')
    cfgs[name] = dict(e.split('=', 1) for e in envs.split(',') if e)
probs = sys.argv[k + 1:]
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        d = json.loads(l); done.add((d['problem'], d['cfg'], d['rep']))
tasks = [(p, c, r) for r in range(reps) for p in probs for c in cfgs if (p, c, r) not in done]

def one(t):
    p, c, r = t
    env = dict(os.environ, **cfgs[c])
    cmd = [sys.executable, str(BENCH), '--worker', str(CACHE / f'{p}.mps'), '--tl', str(tl), '--threads', str(threads)]
    t0 = time.time()
    try:
        pr = subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=tl * 1.5 + 60)
        res = None
        for line in reversed(pr.stdout.splitlines()):
            if line.startswith('{'):
                res = json.loads(line); break
        if res is None:
            res = {'status': f'crash rc={pr.returncode}', 'stderr': pr.stderr[-800:]}
    except subprocess.TimeoutExpired:
        res = {'status': 'killed'}
    return {'problem': p, 'cfg': c, 'rep': r, 'wall': time.time() - t0, 'result': res}

with ThreadPoolExecutor(jobs) as ex, open(out, 'a') as f:
    for d in ex.map(one, tasks):
        f.write(json.dumps(d) + '\n'); f.flush()
        o = d['result']
        print(f"{d['problem']:22s} {d['cfg']:6s} r{d['rep']} {o.get('status'):>12s} t={o.get('time')} obj={o.get('objective')} bd={o.get('best_bound')} nodes={o.get('nodes')}", flush=True)
