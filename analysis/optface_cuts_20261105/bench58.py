"""58 問 (analysis/cbcimpl_20261029/probs.py) を既定 (def) と試作 (mv・par・mvpar = ENOMOTO_MIP_OPTFACE_*) で解く (analysis/optface_cuts_20261105、2026-11-05。
analysis/fragility_20261105/neos859080/bench58/bench58.py の写しで、設定・種・一時ファイルだけ変えた。4 スレッド・60 秒・1 回ずつ)。

使い方: python analysis/optface_cuts_20261105/bench58.py def,mv <種> [<問題,...>] [<出力名>]  (ログは常に logs/ に残す)
結果は <出力名 (既定 bench58)>.jsonl に 1 行ずつ足す (済んだものは飛ばす)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
HW = {"ENOMOTO_MIP_DIAG_HW_THREADS": "16"}
CFGS = {  # (スレッド数, 足す環境変数)
    "def": (4, {}), "mv": (4, {"ENOMOTO_MIP_OPTFACE_MOVE": "f"}), "par": (4, {"ENOMOTO_MIP_OPTFACE_PARCUTS": "3"}),
    "mvpar": (4, {"ENOMOTO_MIP_OPTFACE_MOVE": "f", "ENOMOTO_MIP_OPTFACE_PARCUTS": "3"}),
}


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


def run_one(p, c, log, seed):
    n, extra = CFGS[c]
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = dict(os.environ, ENOMOTO_MIP_THREADS=str(n), ENOMOTO_T_MIP_SEED=str(seed), **extra)
    if log:
        env["ENOMOTO_MIP_LOG"] = "1"
    errf = Path("/tmp/claude-0/optface_bench58_stderr.txt")
    with open(errf, "w") as ef:
        t0 = time.perf_counter()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                               "--time-limit", "60", "--threads", str(n)], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True)
        mth = mrss = 0
        killed = False
        while pr.poll() is None:
            th, rss = status_of(pr.pid)
            mth, mrss = max(mth, th), max(mrss, rss)
            if time.perf_counter() - t0 > 300:
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
        with gzip.open(D / "logs" / f"{p}_{c}_s{seed}.log.gz", "wt") as g:
            g.write(errf.read_text(errors="replace"))
    return {"problem": p, "cfg": c, "result": res, "wall": wall, "max_threads": mth, "max_rss_kb": mrss, "rc": pr.returncode,
            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}


if __name__ == "__main__":
    cfgs = sys.argv[1].split(",")
    seed = int(sys.argv[2])
    probs = sys.argv[3].split(",") if len(sys.argv) > 3 and sys.argv[3] else PROBS
    name = sys.argv[4] if len(sys.argv) > 4 else f"bench58_s{seed}"
    log = True
    OUT = D / f"{name}.jsonl"
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in OUT.read_text().splitlines()} if OUT.exists() else set()
    for i, p in enumerate(probs):
        order = cfgs if i % 2 == 0 else cfgs[::-1]  # 問題ごとに設定の順を逆にする
        for c in order:
            if (p, c) in done:
                continue
            r = run_one(p, c, log, seed)
            r["seed"] = seed
            with open(OUT, "a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, r["result"].get("status"), round(r["wall"], 1), r["max_threads"], r["max_rss_kb"] // 1024, "MB", flush=True)
