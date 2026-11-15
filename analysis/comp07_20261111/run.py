"""COMP07 (2026-11-11): comp07-2idx を、指定のビルド (PYTHONPATH に入れた wheel の展開先) と設定 (cfg.py の CFGS) と種で、
4 スレッドで解き、res.jsonl に 1 行足し、ENOMOTO_MIP_LOG=1 のログを logs/<ビルド>_<設定>_s<種>_<秒>.log に残す。
ビルドは /tmp/claude-0/py/<ビルド> (maturin build した wheel を pip install --target で展開したもの)。同じ行があれば飛ばす。
使い方: python analysis/comp07_20261111/run.py ビルド 設定 種 秒 [問題]   (並べるときは run_many.py)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
PYROOT = Path(os.environ.get("COMP07_PYROOT", "/tmp/claude-0/py"))


def run(build, cfg, seed, tl, prob="comp07-2idx", out=D / "res.jsonl", logdir=D / "logs"):
    key = (prob, build, cfg, seed, tl)
    if out.exists():
        for l in out.read_text().splitlines():
            r = json.loads(l)
            if (r["problem"], r["build"], r["cfg"], r["seed"], r["tl"]) == key:
                return r
    mps = WT / ".miplib_cache" / f"{prob}.mps"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg])
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", PYTHONPATH=str(PYROOT / build))
    logdir.mkdir(exist_ok=True)
    log = logdir / f"{prob}_{build}_{cfg}_s{seed}_{tl}.log"
    t0 = time.time(); peak = 0.0
    with open(log, "w") as lf:
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl),
                               "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True)
        while pr.poll() is None:
            try:
                peak = max(peak, next(int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:")))
            except (OSError, StopIteration):
                pass
            time.sleep(0.5)
            if time.time() - t0 > tl + 300:
                pr.kill(); break
        out_s, _ = pr.communicate()
    try:
        res = json.loads([l for l in out_s.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    r = {"problem": prob, "build": build, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "max_rss_mb": round(peak),
         "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(out, "a") as f:
        f.write(json.dumps(r) + "\n")
    print(prob, build, cfg, seed, tl, res.get("status"), res.get("objective"), round(res.get("time") or 0, 1), flush=True)
    return r


if __name__ == "__main__":
    a = sys.argv[1:]
    run(a[0], a[1], int(a[2]), int(a[3]), *(a[4:5]))
