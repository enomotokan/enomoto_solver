"""根だけの比べ (1 スレッド・種 1、時間の上限 TL 秒): python root.py [TL] [並べる数]。
問題 PROBS × 設定 (cfg.py) を prof_run.py (ENOMOTO_MIP_LOG=1) で解き、ログを logs/<設定>_<問題>.log に残す (既にあれば回さない)。
根の上限の判定は既定で手間 (ENOMOTO_MIP_TICKS の w) で測るので、1 スレッドなら並べて回しても根のカットのループは同じになる。
集計は rootsum.py。"""
import os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
PROBS = "p200x1188c mc11 beasleyC3 10teams exp-1-500-5-5 nw04 qiu timtab1".split()
TL = sys.argv[1] if len(sys.argv) > 1 else "60"
NJ = int(sys.argv[2]) if len(sys.argv) > 2 else 3
def one(job):
    p, c = job
    log = D / "logs" / f"{c}_{p}.log"
    if log.exists() and "@@RESULT" in log.read_text():
        return
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[c]); env["ENOMOTO_T_MIP_SEED"] = "1"
    subprocess.run([sys.executable, str(R / "analysis/smallprob_overhead_20261028/prof_run.py"), "ours", str(R / ".miplib_cache" / f"{p}.mps"), str(log),
                    "--threads", "1", "--time-limit", TL], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(c, p, log.read_text().splitlines()[-1], flush=True)
with ThreadPoolExecutor(NJ) as ex:
    list(ex.map(one, [(p, c) for p in PROBS for c in CFGS]))
