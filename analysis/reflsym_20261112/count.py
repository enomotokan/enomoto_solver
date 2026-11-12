"""前処理の後の問題で、反転を含む対称性を数える (ENOMOTO_MIP_REFLSYM_COUNT=stop、解かない。analysis/reflsym_20261112.md)。
python count.py <出力.jsonl> <問題の一覧のファイル> [値 (既定 stop、r なら再スタートの後も数えて解き続ける)] [時間 (既定 120)]"""
import json, os, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
out, lst = sys.argv[1], sys.argv[2]
val = sys.argv[3] if len(sys.argv) > 3 else "stop"
tl = sys.argv[4] if len(sys.argv) > 4 else "120"
done = set()
if os.path.exists(out):
    done = {json.loads(l)["name"] for l in open(out)}
for name in open(lst).read().split():
    if name in done:
        continue
    env = dict(os.environ, ENOMOTO_MIP_REFLSYM_COUNT=val, ENOMOTO_MIP_THREADS="1")
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{name}.mps"),
                            "--time-limit", tl, "--threads", "1"], env=env, capture_output=True, text=True, timeout=900)
        lines = [l for l in r.stderr.splitlines() if l.startswith("REFLSYMCOUNT")]
    except subprocess.TimeoutExpired:
        lines = ["timeout"]
    with open(out, "a") as f:
        f.write(json.dumps({"name": name, "lines": lines, "wall": round(time.time() - t0, 1)}) + "\n")
    print(name, lines[:1], flush=True)
