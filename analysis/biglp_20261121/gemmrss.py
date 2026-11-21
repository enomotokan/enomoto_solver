"""private-gemm のスレッドごとの作業領域 (L3 × 2 = 532480 kB 前後の無名の写像) が RSS をどれだけ使うかを見る (2026-11-21、直す案 3)。
使い方: python gemmrss.py 問題 スレッド数 秒 出力の名前 [環境変数=値...]
2 秒ごとに /proc/<pid>/smaps を読み、大きさ 530000〜536000 kB の無名の写像の数と RSS の合計 (MB)、全体の VmRSS を
logs/<出力の名前>.gemm に書く (経過秒 写像の数 その RSS 全体の RSS、続けて 200 MB 以上の無名の写像の 大きさ/RSS (MB) の並び。隣り合う写像は核が 1 つにまとめることがある)。stderr は logs/<出力の名前>.log (行ごとに経過秒)。"""
import os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
p, th, tl, name = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED="1")
for kv in sys.argv[5:]:
    k, v = kv.split("=", 1); env[k] = v
cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", tl, "--threads", th]
t0 = time.time()
pr = subprocess.Popen(cmd, cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
def errpump():
    with open(D / "logs" / f"{name}.log", "w") as f:
        for l in pr.stderr:
            f.write(f"[{time.time() - t0:8.2f}] {l}")
threading.Thread(target=errpump, daemon=True).start()
big = []
def scan(pid):
    n = rss = 0; size = None; tot = 0; big.clear()
    try:
        with open(f"/proc/{pid}/smaps") as f:
            for l in f:
                if l[0] in "0123456789abcdef" and "-" in l.split()[0]:
                    size = None; cur = l
                elif l.startswith("Size:"):
                    size = int(l.split()[1])
                elif l.startswith("Rss:"):
                    r = int(l.split()[1]); tot += r
                    if size and size >= 200000 and len(cur.split()) < 6:
                        big.append((size // 1024, r // 1024))
                    if size and 530000 <= size <= 536000 and len(cur.split()) < 6:
                        n += 1; rss += r
    except OSError:
        pass
    return n, rss / 1024, tot / 1024
with open(D / "logs" / f"{name}.gemm", "w") as fo:
    while pr.poll() is None:
        n, r, tot = scan(pr.pid)
        fo.write(f"{time.time() - t0:.1f} {n} {r:.0f} {tot:.0f} " + " ".join(f"{a}/{b}" for a, b in big) + "\n"); fo.flush()
        time.sleep(2)
out = pr.stdout.read()
print([l for l in out.splitlines() if l.startswith("{")][-1:])
