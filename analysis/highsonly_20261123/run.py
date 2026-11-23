"""HiGHS だけが解ける問題の調べ: 1 回ずつ worker を回してログを残す。

    python analysis/highsonly_20261123/run.py 名札 種類 問題[,問題..] [スレッド] [上限秒] [追加]

- 種類: ours (当方、ENOMOTO_MIP_LOG=1 で経過を標準エラーへ) / highs (highspy、ログつき) / scip。
- 追加: ours なら `ENOMOTO_X=1;ENOMOTO_Y=2` (環境変数)、highs なら `mip_allow_restart=false,...` (ENOMOTO_BENCH_HIGHS_OPT)。
- 結果は res.jsonl に 1 行 (名札・種類・問題・スレッド・上限・追加・結果・壁時計)。ログは logs/<名札>_<問題>.log.gz。
- 同じ (名札, 問題) が res.jsonl にあれば飛ばす。ENOMOTO_ で始まる環境変数は外してから足す。種は 1。
"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path

D = Path(__file__).resolve().parent
REPO = D.parents[1]
RES = D / "res.jsonl"


def done():
    if not RES.exists():
        return set()
    return {(r["tag"], r["problem"]) for r in map(json.loads, RES.read_text().splitlines()) if r.get("result")}


def main():
    tag, kind, probs = sys.argv[1], sys.argv[2], sys.argv[3].split(",")
    th = int(sys.argv[4]) if len(sys.argv) > 4 else 1
    tl = float(sys.argv[5]) if len(sys.argv) > 5 else 600
    extra = sys.argv[6] if len(sys.argv) > 6 else ""
    seen = done()
    for p in probs:
        if (tag, p) in seen:
            print("済み", tag, p)
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env["ENOMOTO_MIP_THREADS"] = str(th)
        env["ENOMOTO_T_MIP_SEED"] = "1"
        env["ENOMOTO_BENCH_SCIP_SEED"] = "1"
        if kind == "ours":
            env["ENOMOTO_MIP_LOG"] = "1"
            for kv in filter(None, extra.split(";")):
                k, v = kv.split("=", 1)
                env[k] = v
        elif kind == "highs":
            env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
            env["ENOMOTO_BENCH_HIGHS_OPT"] = ",".join(filter(None, ["random_seed=1", extra]))
        elif kind == "scip":
            env["ENOMOTO_BENCH_SCIP_LOG"] = "1"
        mps = REPO / ".miplib_cache" / f"{p}.mps"
        cmd = [sys.executable, str(REPO / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps),
               "--time-limit", str(tl), "--threads", str(th)]
        (D / "logs").mkdir(exist_ok=True)
        t0 = time.time()
        pr = subprocess.run(cmd, env=env, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                            timeout=tl + 600)
        wall = time.time() - t0
        out = pr.stdout
        with gzip.open(D / "logs" / f"{tag}_{p}.log.gz", "wt") as f:
            f.write(out)
        res = None
        for line in reversed(out.splitlines()):
            if line.startswith("{"):
                try:
                    res = json.loads(line)
                    break
                except ValueError:
                    pass
        row = {"tag": tag, "kind": kind, "problem": p, "threads": th, "tl": tl, "extra": extra, "result": res,
               "wall": round(wall, 1), "rc": pr.returncode, "at": time.strftime("%Y-%m-%d %H:%M:%S")}
        with open(RES, "a") as f:
            f.write(json.dumps(row) + "\n")
        r = res or {}
        print(tag, p, r.get("status"), r.get("objective"), r.get("bound"), r.get("nodes"), round(r.get("time") or wall, 1), flush=True)


main()
