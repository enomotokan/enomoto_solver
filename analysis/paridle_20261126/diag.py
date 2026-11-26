"""遊び・重複探索の診断の回し方: python diag.py T K N (P の 58 問の K 番目から N おきに、種 1・60 秒・T スレッドで one.py を回す)。
4 スレッドは 1 本ずつ (diag.py 4 0 1)、1 スレッドは 3 本並べる (diag.py 1 0 3・1 1 3・1 2 3)。済んだ問題 (logs/log_問題_tT.out がある) は飛ばす。"""
import subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
T, K, N = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
for p in PROBS[K::N]:
    if (D / "logs" / f"log_{p}_t{T}.out").exists():
        continue
    subprocess.run([sys.executable, str(D / "one.py"), p, T, "1", "60", f"t{T}"])
