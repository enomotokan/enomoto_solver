"""rocI-4-11 の誤答 (glc) の切り分け用: 既定で解いて解を書き出す (ENOMOTO_MIP_DEBUG_SOL・START_FILE の形、元の問題の列の順に 1 行 1 値)。
python getsol.py <問題> <出力> [秒] [スレッド]"""
import sys, json
from pathlib import Path
WT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
from enomoto_solver import benchmark_highs as bh  # noqa: E402
p, out = sys.argv[1], sys.argv[2]
tl = float(sys.argv[3]) if len(sys.argv) > 3 else 600; th = int(sys.argv[4]) if len(sys.argv) > 4 else 4
m, _, _, _ = rb._read_our_model_compat(bh, rb.fetch(p, WT / ".miplib_cache"))
r = m.solve(None, False, tl, None, None, th)
x = r["x"]
Path(out).write_text("\n".join(repr(float(v)) for v in x) + "\n")
print(json.dumps({"status": r["status"], "objective": r["objective"], "n": len(x)}))
