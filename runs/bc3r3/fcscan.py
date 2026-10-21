"""40 問で固定費用型 (ノード用、行 0.5・列 0.9) の判定を見る (2 秒)。"""
import os, re, subprocess, sys
from pathlib import Path
REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'scripts'))
from run_miplib_benchmark import PROBLEMS
for n in PROBLEMS:
    mps = REPO / '.miplib_cache' / f'{n}.mps'
    code = f"from pathlib import Path\nfrom enomoto_solver import benchmark_highs as bh\n_, lp = bh._load_lp(Path({str(mps)!r}))\nm, _, _ = bh._build_our_model(lp)\nm.solve(None, False, 2, None, None, 4, log=True)"
    p = subprocess.run([sys.executable, '-c', code], capture_output=True, text=True, env=dict(os.environ, ENOMOTO_MIP_THREADS='4', ENOMOTO_MIP_LOG='1'), timeout=300, cwd='/tmp')
    m = re.search(r'fixed-charge check: vb rows (\d+) of (\d+), continuous with vb (\d+) of (\d+)', p.stderr)
    if m:
        a, b, c, d = map(int, m.groups())
        print(n, a, b, c, d, 'FC' if a >= 0.5 * b and d > 0 and c >= 0.9 * d else '', flush=True)
    else:
        print(n, '-', flush=True)
