"""長い時間の上限 (既定 600 秒) で 1 問を ENOMOTO / HiGHS の一方で解き、最大 RSS・ログの大きさの推移を取る。

/usr/bin/time が無い機械なので、ワーカー (scripts/run_miplib_benchmark.py --worker-*) を子プロセスで起こし、
- 10 秒ごとに子の木 (python とその子) の RSS の和と、標準エラー (ENOMOTO_MIP_LOG=1 のログ) のファイルの大きさを記録
- 終わりに resource.getrusage(RUSAGE_CHILDREN).ru_maxrss (最大 RSS) と経過時間を記録
する。結果は 1 行の JSON で --out に書き足す。
使い方: python run_long.py ours|highs NAME [--time-limit 600] [--threads 4] [--out long.jsonl] [--logdir DIR]
"""
import argparse, json, os, resource, subprocess, sys, time
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
BENCH = REPO / "scripts" / "run_miplib_benchmark.py"


def tree_rss_kb(pid):
    pids, todo = [], [pid]
    while todo:
        p = todo.pop()
        pids.append(p)
        try:
            for t in os.listdir(f"/proc/{p}/task"):
                with open(f"/proc/{p}/task/{t}/children") as f:
                    todo += [int(c) for c in f.read().split()]
        except OSError:
            pass
    tot = 0
    for p in pids:
        try:
            with open(f"/proc/{p}/status") as f:
                for line in f:
                    if line.startswith("VmRSS:"):
                        tot += int(line.split()[1])
        except OSError:
            pass
    return tot


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("kind", choices=["ours", "highs"])
    ap.add_argument("name")
    ap.add_argument("--time-limit", type=float, default=600)
    ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--out", default="long.jsonl")
    ap.add_argument("--logdir", default=".")
    ap.add_argument("--every", type=float, default=10)
    a = ap.parse_args()
    sys.path.insert(0, str(REPO / "scripts"))
    import run_miplib_benchmark as rb
    mps = rb.fetch(a.name, REPO / ".miplib_cache")
    env = dict(os.environ)
    if a.threads > 1:
        env["ENOMOTO_MIP_THREADS"] = str(a.threads)
    if a.kind == "ours":
        env["ENOMOTO_MIP_LOG"] = "1"
    tag = f"{a.name}_{a.kind}_{int(a.time_limit)}"
    errp = Path(a.logdir) / f"{tag}.stderr"
    outp = Path(a.logdir) / f"{tag}.stdout"
    cmd = [sys.executable, str(BENCH), f"--worker-{a.kind}", str(mps), "--time-limit", str(a.time_limit), "--threads", str(a.threads)]
    samples = []
    t0 = time.perf_counter()
    with open(errp, "wb") as ef, open(outp, "wb") as of:
        p = subprocess.Popen(cmd, stdout=of, stderr=ef, env=env)
        killed = False
        while True:
            try:
                p.wait(timeout=a.every)
                break
            except subprocess.TimeoutExpired:
                pass
            el = time.perf_counter() - t0
            samples.append([round(el, 1), tree_rss_kb(p.pid), errp.stat().st_size])
            if el > a.time_limit * 1.5 + 60:
                p.kill(); p.wait(); killed = True
                break
    wall = time.perf_counter() - t0
    ru = resource.getrusage(resource.RUSAGE_CHILDREN)
    res = None
    for line in reversed(outp.read_text().splitlines()):
        if line.startswith("{"):
            res = json.loads(line)
            break
    rec = {"problem": a.name, "solver": a.kind, "time_limit": a.time_limit, "threads": a.threads,
           "result": res, "rc": p.returncode, "killed": killed, "wall": wall,
           "maxrss_mb": ru.ru_maxrss / 1024, "cpu_s": ru.ru_utime + ru.ru_stime,
           "log_bytes": errp.stat().st_size, "samples": samples,
           "at": time.strftime("%H:%M:%S")}
    with open(a.out, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps({k: v for k, v in rec.items() if k != "samples"}))


if __name__ == "__main__":
    main()
