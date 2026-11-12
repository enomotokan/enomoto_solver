"""ROOTCAP: 1 問を ENOMOTO (4 スレッド) で 1 回解き、時刻つきのログと結果を残す。
使い方: python run1.py 問題 設定 種 秒   (設定は CFGS の名前。def = 既定)
出力: res.jsonl に 1 行、logs/<問題>.<設定>.s<種>.t<秒>.log.gz (各行の頭にプロセスの開始からの経過秒)。
HiGHS は 設定 = highs (highspy 1.15.1、mip_rel_gap=0)。"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "h": {"ENOMOTO_MIP_ROOTCAP": "h"},
    "p": {"ENOMOTO_MIP_ROOTCAP": "p"},
    "d": {"ENOMOTO_MIP_ROOTCAP": "d"},
    "hp": {"ENOMOTO_MIP_ROOTCAP": "hp"},
    "hpd": {"ENOMOTO_MIP_ROOTCAP": "hpd"},
    "hd": {"ENOMOTO_MIP_ROOTCAP": "hd"},
    "hdw": {"ENOMOTO_MIP_ROOTCAP": "hdw"},
    "q": {"ENOMOTO_MIP_ROOTCAP": "q"},
}
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
def main():
    prob, cfg, seed, tl = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4])
    mps = rb.fetch(prob, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    solver = "highs" if cfg == "highs" else "ours"
    if solver == "ours":
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
        env.update(CFGS[cfg])
    else:
        env.update(ENOMOTO_BENCH_HIGHS_LOG="1")
    log = D / "logs" / f"{prob}.{cfg}.s{seed}.t{int(tl)}.log.gz"
    t0 = time.time(); peak = 0.0; last = [""]
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{solver}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    def pump():
        with gzip.open(log, "wt") as g:
            for line in pr.stdout:
                if line.startswith("{"): last[0] = line
                g.write(f"{time.time() - t0:8.2f} {line}")
    th = threading.Thread(target=pump); th.start()
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); break
    th.join()
    try: res = json.loads(last[0])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    row = {"problem": prob, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(D / "res.jsonl", "a") as f:
        f.write(json.dumps(row) + "\n")
    print(prob, cfg, seed, res.get("status"), res.get("time"), res.get("objective"), round(peak), flush=True)
main()
