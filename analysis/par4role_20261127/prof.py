"""4T の 1 回の求解を、スレッドごとに外から見る: python prof.py 問題 [スレッド数=4] [上限秒=600]
- /proc/<pid>/task/*/stat を 0.25 秒ごとに読み、スレッドごとの CPU 時間 (utime+stime) の増えを記録する (止めない)。
- (gdb の標本は使わない: つなぐとスレッドを作るたびに止まり、時間が 1.8 倍に乱れた)
出力: logs/<問題>_t<N>.log (ENOMOTO_MIP_LOG=1 の標準エラー)、logs/<問題>_t<N>.cpu.jsonl。
PROF_ENV="K=V ..." で環境変数を足す (例 ENOMOTO_PROF_TREE_PAR=1)。PROF_TAG は出力名の接尾辞。"""
import json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
p = sys.argv[1]; th = int(sys.argv[2]) if len(sys.argv) > 2 else 4; tl = sys.argv[3] if len(sys.argv) > 3 else "600"
tag = os.environ.get("PROF_TAG", "")
L = D / "logs"; L.mkdir(exist_ok=True); base = L / f"{p}_t{th}{tag}"
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_") or k.startswith("ENOMOTO_MIP_") and k in os.environ.get("PROF_KEEP", "").split(",")}
env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"})
for kv in os.environ.get("PROF_ENV", "").split():
    k, v = kv.split("=", 1); env[k] = v
err = open(f"{base}.log", "w")
pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", str(th)],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True)
t0 = time.time(); HZ = os.sysconf("SC_CLK_TCK")
cpu = open(f"{base}.cpu.jsonl", "w")
def poll():
    while pr.poll() is None:
        snap = {}
        try:
            for t in os.listdir(f"/proc/{pr.pid}/task"):
                try:
                    s = open(f"/proc/{pr.pid}/task/{t}/stat").read(); r = s.rsplit(")", 1)[1].split()
                    snap[t] = (int(r[11]) + int(r[12])) / HZ
                except OSError: pass
        except OSError: break
        cpu.write(json.dumps({"t": round(time.time() - t0, 3), "cpu": snap}) + "\n"); cpu.flush()
        time.sleep(0.25)
threading.Thread(target=poll, daemon=True).start()
out, _ = pr.communicate()
res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
print(json.dumps({"problem": p, "th": th, "tag": tag, "result": res, "wall": round(time.time() - t0, 1)}), flush=True)
