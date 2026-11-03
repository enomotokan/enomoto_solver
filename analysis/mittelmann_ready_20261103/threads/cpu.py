"""1・2 スレッドで CPU 秒 ÷ 壁時計を測る (並列の部分が切れているか)。使い方: python cpu.py <問題,...> <スレッド数,...>"""
import json, os, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[3]
for p in sys.argv[1].split(","):
    for n in sys.argv[2].split(","):
        t0 = time.perf_counter()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                               "--time-limit", "60", "--threads", n], cwd=WT, env=dict(os.environ, ENOMOTO_MIP_THREADS=n), stdout=subprocess.PIPE, text=True)
        _, st, ru = os.wait4(pr.pid, 0)
        wall = time.perf_counter() - t0
        res = json.loads([l for l in pr.stdout.read().splitlines() if l.startswith("{")][-1])
        cpu = ru.ru_utime + ru.ru_stime
        print(json.dumps({"problem": p, "threads": int(n), "status": res["status"], "time": round(res["time"], 2), "wall": round(wall, 2), "cpu": round(cpu, 2), "cpu_per_wall": round(cpu / wall, 2)}), flush=True)
