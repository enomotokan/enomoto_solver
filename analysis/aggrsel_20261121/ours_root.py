"""当方の根だけ: python ours_root.py 並べる数 [札 ...]。問題 PROBS (scip_root.py と同じ) × 札 (VARS) を 1 スレッド・NODE_LIMIT=1 で、
prof_run.py (ENOMOTO_MIP_LOG=1) に ENOMOTO_MIP_DEBUG_SEPPROF (分離器ごとの候補・採った本数) と ENOMOTO_MIP_GAP4_PATHDIAG (path_highs の経路の数) と再スタートなし (SCIP の maxrestarts=0 に合わせる) を
つけて解き、ログを logs/ours_<問題>_<札>.log に残す (既にあれば回さない)。集計は sum_root.py。"""
import os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
sys.path.insert(0, str(D))
from scip_root import PROBS  # noqa: E402
DIAG = {"ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_DEBUG_SEPPROF": "1", "ENOMOTO_MIP_GAP4_PATHDIAG": "1", "ENOMOTO_MIP_CUTSTOP_LOG": "1", "ENOMOTO_MIP_NO_RESTART": "1"}
VARS = {
    "def": {},
    "nopath": {"ENOMOTO_MIP_NO_PATH_AGG": "1"},
    "old": {"ENOMOTO_MIP_PATH_OLD": "1"},
    "highs": {"ENOMOTO_MIP_PATH_HIGHS_ONLY": "1"},
}
try:
    from cfg import CFGS  # 試作の設定 (あれば)
    VARS.update(CFGS)
except ImportError:
    pass
def one(job):
    p, tag = job
    log = D / "logs" / f"ours_{p}_{tag}.log"
    if log.exists() and "@@RESULT" in log.read_text():
        return
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(DIAG); env.update(VARS[tag]); env["ENOMOTO_T_MIP_SEED"] = "1"
    subprocess.run([sys.executable, str(R / "analysis/smallprob_overhead_20261028/prof_run.py"), "ours", str(R / ".miplib_cache" / f"{p}.mps"), str(log),
                    "--threads", "1", "--time-limit", "900"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(tag, p, log.read_text().splitlines()[-1][:200], flush=True)
if __name__ == "__main__":
    nj = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    tags = sys.argv[2:] or list(VARS)
    (D / "logs").mkdir(exist_ok=True)
    with ThreadPoolExecutor(nj) as ex:
        list(ex.map(one, [(p, t) for t in tags for p in PROBS]))
