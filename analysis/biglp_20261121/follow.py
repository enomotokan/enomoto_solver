"""追いの調べ (2026-11-21)。既定のまま、計測は 1 本ずつ。
- dive: sing44 4 スレッド 600 秒。ログに「fractional diving cap」が出たら、潜りが終わるまで 20 秒ごとに gdb で全スレッドのスタックを取る
  (logs/sing44_dive_gdb_<k>.txt)。ログは logs/sing44_dive.log (経過秒つき)。
- heap: rail01 4 スレッド 600 秒を heaptrack の下で回す (logs/rail01_ht.zst → heaptrack_print の要約を logs/rail01_ht.txt)。RSS も logs/rail01_ht.rss。
使い方: python follow.py dive|heap"""
import os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]; L = D / "logs"
what = sys.argv[1]
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1")
p = "sing44" if what == "dive" else "rail01"
cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "600", "--threads", "4"]
if what == "heap":
    cmd = ["heaptrack", "-o", str(L / "rail01_ht")] + cmd
t0 = time.time()
pr = subprocess.Popen(cmd, cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
state = {"dive": False, "done": False}
def pump():
    with open(L / f"{p}_{'dive' if what == 'dive' else 'ht'}.log", "w") as f:
        for l in pr.stderr:
            f.write(f"[{time.time() - t0:8.2f}] {l}"); f.flush()
            if "fractional diving cap" in l: state["dive"] = True
            if "root heuristic fractional diving:" in l: state["done"] = True
threading.Thread(target=pump, daemon=True).start()
def target_pid():
    # heaptrack の下ではその子の python を探す
    if what != "heap": return pr.pid
    out = subprocess.run(["pgrep", "-P", str(pr.pid)], capture_output=True, text=True).stdout.split()
    return int(out[0]) if out else pr.pid
k = 0; last = 0
with open(L / f"{p}_{'dive' if what == 'dive' else 'ht'}.rss", "w") as fr:
    while pr.poll() is None:
        pid = target_pid()
        try: rss = next(int(l.split()[1]) // 1024 for l in open(f"/proc/{pid}/status") if l.startswith("VmRSS:"))
        except Exception: rss = 0  # noqa: BLE001
        fr.write(f"{time.time() - t0:.1f} {rss}\n"); fr.flush()
        if what == "dive" and state["dive"] and not state["done"] and time.time() - last > 20 and k < 12:
            last = time.time(); k += 1
            r = subprocess.run(["gdb", "-batch", "-p", str(pid), "-ex", "thread apply all bt 30"], capture_output=True, text=True, timeout=120)
            (L / f"sing44_dive_gdb_{k}.txt").write_text(f"t={time.time() - t0:.1f}\n" + r.stdout[-400000:])
        time.sleep(1)
print(pr.stdout.read().strip().splitlines()[-1:], flush=True)
