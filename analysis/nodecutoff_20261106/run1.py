"""打ち切り値の計数・試作の 1 回の計測 (2026-11-06)。
1 問を 4 スレッド・TL 秒で解き、結果と stderr の `MIP: nodecut` の行を <出力>.jsonl に 1 行で書く。
使い方: python run1.py 出力.jsonl 設定 問題 秒 rep
設定: base (既定 + 計数 STAT)、live (試作 LIVE + STAT)、live8 / live128 (読み直しの間隔)、plain (環境変数なし)、plainlive (LIVE だけ)
"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "base": {"ENOMOTO_MIP_NODECUT_STAT": "1"},
    "live": {"ENOMOTO_MIP_NODECUT_STAT": "1", "ENOMOTO_MIP_NODECUT_LIVE": "1"},
    "live8": {"ENOMOTO_MIP_NODECUT_STAT": "1", "ENOMOTO_MIP_NODECUT_LIVE": "1", "ENOMOTO_T_MIP_NODECUT_LIVE_EVERY": "8"},
    "live128": {"ENOMOTO_MIP_NODECUT_STAT": "1", "ENOMOTO_MIP_NODECUT_LIVE": "1", "ENOMOTO_T_MIP_NODECUT_LIVE_EVERY": "128"},
    "plain": {},
    "plainlive": {"ENOMOTO_MIP_NODECUT_LIVE": "1"},
}
out, cfg, p, tl, rep = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", **CFGS[cfg])
t0 = time.time()
try:
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 2 + 120)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    err = r.stderr
except Exception as e:  # noqa: BLE001
    res = {"status": "crash", "error": str(e)[:300]}
    err = ""
nc = {}
for l in err.splitlines():
    m = re.match(r"MIP: nodecut (\w+): (.*)", l)
    if m:
        nc[m.group(1)] = [int(x) for x in re.findall(r"(?<![\w.])(\d+)(?![\w.])", m.group(2))]
with open(out, "a") as f:
    f.write(json.dumps({"problem": p, "cfg": cfg, "rep": rep, "tl": tl, "wall": round(time.time() - t0, 1), "result": res, "nodecut": nc}) + "\n")
print(p, cfg, rep, res.get("status"), round(res.get("time") or 0, 1), res.get("nodes"), {k: v[:6] for k, v in nc.items()}, flush=True)
