"""再スタートで持ち込むカットの試作を、設定ごとに 1 問ずつ順に解く (4 スレッド・60 秒。同時には回さない)。

使い方: python run.py <出力 jsonl> <設定名> <回数> <問題 ...> [--env K=V ...] [--log-dir DIR]
各回の stderr は --log-dir があれば <DIR>/<問題>_<設定名>_<回>.log に残す (診断の行を後で拾うため)。"""
import argparse, json, os, subprocess, sys, time
from pathlib import Path

here = Path(__file__).resolve().parent
repo = here.parents[1]
sys.path.insert(0, str(repo / "scripts"))
import run_miplib_benchmark as rb

ap = argparse.ArgumentParser()
ap.add_argument("out")
ap.add_argument("cfg")
ap.add_argument("reps", type=int)
ap.add_argument("problems", nargs="+")
ap.add_argument("--env", nargs="*", default=[])
ap.add_argument("--log-dir")
ap.add_argument("--time-limit", type=float, default=60.0)
ap.add_argument("--threads", type=int, default=4)
ap.add_argument("--rep-start", type=int, default=0)  # 設定を交互に 1 回ずつ回すとき、記録する回の番号の始まり
ap.add_argument("--skip-done", action="store_true")  # 出力に同じ (問題, 設定, 回) がもうあれば解かない (止まった計測の続きから)
a = ap.parse_args()
Path(a.out).parent.mkdir(parents=True, exist_ok=True)
done = set()
if a.skip_done and Path(a.out).exists():
    for l in open(a.out, encoding="utf-8"):
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["cfg"], r["rep"]))
extra = dict(e.split("=", 1) for e in a.env)
env = dict(os.environ, ENOMOTO_MIP_THREADS=str(a.threads), **extra)
with open(a.out, "a") as out:
    for rep in range(a.rep_start, a.rep_start + a.reps):
        for n in a.problems:
            if (n, a.cfg, rep) in done:
                continue
            mps = rb.fetch(n, repo / ".miplib_cache")
            cmd = [sys.executable, str(repo / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                   "--time-limit", str(a.time_limit), "--threads", str(a.threads)]
            t = time.time()
            try:
                p = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=a.time_limit * 1.5 + 60)
                lines = [l for l in p.stdout.splitlines() if l.startswith("{")]
                r = json.loads(lines[-1]) if lines else {"status": f"crash rc={p.returncode}", "stderr": p.stderr[-500:]}
                err = p.stderr
            except subprocess.TimeoutExpired as e:
                r = {"status": "killed"}
                err = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
            wall = time.time() - t
            if a.log_dir:
                Path(a.log_dir).mkdir(parents=True, exist_ok=True)
                (Path(a.log_dir) / f"{n}_{a.cfg}_{rep}.log").write_text(err)
            out.write(json.dumps({"problem": n, "cfg": a.cfg, "rep": rep, "wall": wall, "result": r}) + "\n")
            out.flush()
            print(f"{n:22s} {a.cfg} {rep} {r.get('status')} obj={r.get('objective')} t={r.get('time')}", flush=True)
