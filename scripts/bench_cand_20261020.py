"""HiGHS だけが解ける 16 問の調査 (2026-10-20) の候補の計測: MIPLIB 40 問と 16 問を、問題 1 問ごとに再開できる形で解く。

HiGHS の結果は基準 (相対ギャップ 0 で測り直した benchmarks/*_4t_60s_20261020_bc89ad1_highsgap0.json) を使い回し、ENOMOTO だけ解き直す (4 スレッド・60 秒)。
入っている enomoto_solver をそのまま使う。結果は benchmarks/{miplib_results,highsonly16}_4t_60s_20261020_<SHA>_<名前>_r<回>.json、
SHA は今の HEAD。進み具合は benchmarks/cand_logs/<名前>_r<回>.<40|16>.log。

使い方: python scripts/bench_cand_20261020.py <名前> <回> [ENV=VAL ...]
"""
import os, shutil, subprocess, sys
from pathlib import Path
REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / 'scripts'))
from run_miplib_benchmark import PROBLEMS
P16 = ("nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 "
       "uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1").split()
# 結果をコミットすると HEAD が進むので、`ENOMOTO_BENCH_SHA` でファイル名の SHA を固定できる (ビルドしたコミットを渡す)
SHA = os.environ.get('ENOMOTO_BENCH_SHA') or subprocess.run(['git', 'rev-parse', '--short=7', 'HEAD'], cwd=REPO, capture_output=True, text=True).stdout.strip() or 'unknown'
name, run = sys.argv[1], sys.argv[2]
env = dict(os.environ, **dict(a.split('=', 1) for a in sys.argv[3:]))
logdir = REPO / 'benchmarks' / 'cand_logs'
logdir.mkdir(parents=True, exist_ok=True)
for suite, probs, base in (('40', PROBLEMS, 'miplib_results'), ('16', P16, 'highsonly16')):
    out = REPO / 'benchmarks' / f'{base}_4t_60s_20261020_{SHA}_{name}_r{run}.json'
    if not out.exists():
        shutil.copy(REPO / 'benchmarks' / f'{base}_4t_60s_20261020_bc89ad1_highsgap0.json', out)
    log = logdir / f'{name}_r{run}.{suite}.log'
    done = {l.split()[0] for l in log.read_text().splitlines() if ' ours=' in l} if log.exists() else set()
    for p in probs:
        if p in done:
            continue
        r = subprocess.run([sys.executable, 'scripts/run_miplib_benchmark.py', '--threads', '4', '--time-limit', '60', '--rerun-ours',
                            '--only', p, '--out', str(out)], cwd=REPO, capture_output=True, text=True, env=env)
        line = next((l for l in r.stdout.splitlines() if l.startswith(p + ' ')), None)
        if line is None:
            print(f'{name} r{run} {p} failed: {r.stderr[-500:]}', flush=True)
            continue
        with open(log, 'a') as f:
            f.write(line + '\n')
        print(line, flush=True)
print(f'{name} r{run} ALL DONE', flush=True)
