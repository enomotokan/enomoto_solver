"""gate2 候補 2 の確かめ (2): 再スタートの後のプロービングで初めて CLQPRB の判定に入る問題がないか。
python rscan.py 秒 問題... (clqs、種 1、4 スレッド、ENOMOTO_MIP_LOG=1)。ログの CLQPRB の行と再スタートの行 (と経過秒) を rscan.jsonl に足す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
tl = float(sys.argv[1])
for p in sys.argv[2:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS["clqs"]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
    t0 = time.time(); lines = []
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    final = None
    for line in pr.stdout:
        low = line.lower()
        if "clqprb" in low or ("restart" in low and len(lines) < 200):
            lines.append(f"{time.time() - t0:7.1f} {line.strip()[:220]}")
        if line.startswith("{"):
            try: final = json.loads(line)
            except Exception: pass  # noqa: BLE001
    pr.wait()
    g = [l for l in lines if "CLQPRB=g" in l]
    res = {"problem": p, "tl": tl, "n_g": len(g), "n_restart_lines": sum("restart" in l.lower() for l in lines), "lines": lines[:80],
           "status": (final or {}).get("status"), "time": (final or {}).get("time")}
    with open(D / "rscan.jsonl", "a") as f: f.write(json.dumps(res) + "\n")
    print(p, res["status"], res["time"], "g", len(g), "restart_lines", res["n_restart_lines"], flush=True)
