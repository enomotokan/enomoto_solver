"""pool_bound1 の計測 (2026-11-04、memcap_c_20261104/bench.py の写しで、設定・時間上限・一時ファイルだけ変えた)。
使い方: python bench.py <設定,...> [<問題,...>] [<出力名>] [<時間上限 秒 (既定 60)>]
- 設定は CFGS の名前 (def = 今の既定、worst/est/... = ENOMOTO_MIP_POOL1_DONATE)。4 スレッド。
- 壁時計の時間・最大のスレッド数・最大 RSS を /proc/<pid>/status から 0.2 秒ごとに取る。問題ごとに設定の順を入れ替える。
結果は <出力名 (既定 bench)>.jsonl に 1 行ずつ足す (済んだものは飛ばす)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
HW = {"ENOMOTO_MIP_DIAG_HW_THREADS": "16"}
CFGS = {"def": (4, {})}  # (スレッド数, 足す環境変数)
for _m in ["worst", "est", "deep", "shallow", "alt"]:
    CFGS[_m] = (4, {"ENOMOTO_MIP_POOL1_DONATE": _m})
CFGS["def_memc"] = (4, {"ENOMOTO_MIP_MEMC": "1"})
CFGS["worst_memc"] = (4, {"ENOMOTO_MIP_MEMC": "1", "ENOMOTO_MIP_POOL1_DONATE": "worst"})
TL = 60


def status_of(pid):
    th = rss = 0
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("Threads:"):
                th = int(l.split()[1])
            elif l.startswith("VmRSS:"):
                rss = int(l.split()[1])
    except OSError:
        pass
    return th, rss


def run_one(p, c, log=False):
    n, extra = CFGS[c]
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = dict(os.environ, ENOMOTO_MIP_THREADS=str(n), **extra)
    if log:
        env["ENOMOTO_MIP_LOG"] = "1"
    errf = Path(f"/tmp/claude-0/pool1_bench_stderr_{os.getpid()}.txt")
    with open(errf, "w") as ef:
        t0 = time.perf_counter()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                               "--time-limit", str(TL), "--threads", str(n)], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True)
        mth = mrss = 0
        killed = False
        while pr.poll() is None:
            th, rss = status_of(pr.pid)
            mth, mrss = max(mth, th), max(mrss, rss)
            if time.perf_counter() - t0 > TL * 2 + 240:
                pr.kill(); killed = True
            time.sleep(0.2)
        wall = time.perf_counter() - t0
        out = pr.stdout.read()
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        tail = errf.read_text(errors="replace")[-400:]
        res = {"status": "killed" if killed else "crash", "rc": pr.returncode, "error": tail}
    if log:
        (D / "logs").mkdir(exist_ok=True)
        with gzip.open(D / "logs" / f"{p}_{c}.log.gz", "wt") as g:
            g.write(errf.read_text(errors="replace"))
    return {"problem": p, "cfg": c, "result": res, "wall": wall, "max_threads": mth, "max_rss_kb": mrss, "rc": pr.returncode,
            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}


if __name__ == "__main__":
    cfgs = sys.argv[1].split(",")
    probs = sys.argv[2].split(",") if len(sys.argv) > 2 and sys.argv[2] else PROBS
    name = sys.argv[3] if len(sys.argv) > 3 else "bench"
    TL = float(sys.argv[4]) if len(sys.argv) > 4 else 60
    log = False
    OUT = D / f"{name}.jsonl"
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in OUT.read_text().splitlines()} if OUT.exists() else set()
    for i, p in enumerate(probs):
        order = cfgs if i % 2 == 0 else cfgs[::-1]  # 問題ごとに設定の順を逆にする
        for c in order:
            if (p, c) in done:
                continue
            r = run_one(p, c, log)
            with open(OUT, "a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, r["result"].get("status"), round(r["wall"], 1), r["max_threads"], r["max_rss_kb"] // 1024, "MB", flush=True)
