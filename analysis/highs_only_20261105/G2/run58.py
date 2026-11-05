"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定と試作の設定で解き、res58.jsonl に書く (rep 付き)。
問題ごと・rep ごとに設定の順を入れ替える。使い方: python analysis/highs_only_20261105/G2/run58.py rep 設定名..."""
import json, os, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
CFGS = {
    "def": {},
    "ho2": {"ENOMOTO_MIP_HO2_IMPLINT": "1", "ENOMOTO_MIP_HO2_GOBJ": "1"},
}
import subprocess  # noqa: E402


def run(mps):
    """rb.run_worker と同じ (4 スレッド・60 秒) に、ENOMOTO_MIP_LOG=1 の詳細から木の中の再スタートの行を数えて足す (両方の設定で同じログ。`ENOMOTO_MIP_TREE_RESTART_LOG` は判定の途中を出すだけ)。"""
    env = dict(os.environ, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_TREE_RESTART_LOG="1")
    cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=60 * 1.5 + 60, env=env)
    except subprocess.TimeoutExpired:
        return {"status": "killed", "objective": None, "bound": None, "nodes": None, "time": None}
    r = None
    for line in reversed(p.stdout.splitlines()):
        if line.startswith("{"):
            r = json.loads(line); break
    if r is None:
        return {"status": f"crash rc={p.returncode}", "objective": None, "bound": None, "nodes": None, "time": None, "stderr": p.stderr[-2000:]}
    r["mip_warnings"] = p.stderr.count("MIP: WARNING")
    r["tree_restarts"] = [l.split("TREE restart: ")[1].split(" newly")[0] for l in p.stderr.splitlines() if "MIP: TREE restart:" in l]
    r["ho2_step"] = any("objective step with implied integers: Some" in l and "(without: None)" in l for l in p.stderr.splitlines())
    r["ho2_gobj"] = sum("HO2_GOBJ: cutoff" in l for l in p.stderr.splitlines())
    r["treer_skips"] = sum("TREER" in l and "skipped" in l for l in p.stderr.splitlines())
    return r


rep = int(sys.argv[1])
names = sys.argv[2:] or ["def", "ho2"]
OUT = D / "res58.jsonl"
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
for i, p in enumerate(PROBS):
    mps = rb.fetch(p, WT / ".miplib_cache")
    order = names if (i + rep) % 2 == 0 else names[::-1]
    for c in order:
        os.environ.clear(); os.environ.update(base); os.environ.update(CFGS[c])
        t0 = time.time()
        r = run(mps)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": r, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(rep, p, c, r.get("status"), r.get("time"), r.get("objective"), flush=True)
