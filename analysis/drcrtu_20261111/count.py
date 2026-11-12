"""整数の組の判定で外された組を数える (ENOMOTO_MIP_FOLD=<値>c、解かない)。TU に落ちた行列を dump/<問題>_<番号>.txt に書き出す。
python count.py <出力.jsonl> <値> <dump のディレクトリか -> <mps ...>"""
import json, os, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
out, val, dump = sys.argv[1], sys.argv[2], sys.argv[3]
done = set()
if os.path.exists(out):
    done = {json.loads(l)["name"] for l in open(out)}
for mps in sys.argv[4:]:
    name = Path(mps).name.split(".")[0]
    if name in done:
        continue
    env = dict(os.environ, ENOMOTO_MIP_FOLD=val + "c")
    if dump != "-":
        os.makedirs(dump, exist_ok=True)
        env.update(ENOMOTO_T_FOLD_TUDUMP=dump, ENOMOTO_T_FOLD_TUDUMP_TAG=name)
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", mps, "--time-limit", "10", "--threads", "1"],
                           env=env, capture_output=True, text=True, timeout=900)
        lines = [l for l in r.stderr.splitlines() if l.startswith("MIP_FOLD")]
    except subprocess.TimeoutExpired:
        lines = ["timeout"]
    with open(out, "a") as f:
        f.write(json.dumps({"name": name, "val": val, "fold": lines, "wall": round(time.time() - t0, 1)}) + "\n")
    print(name, lines[:2], flush=True)
