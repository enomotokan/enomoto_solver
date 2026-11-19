"""1 回解いてログを残す: python one.py 問題 名前 種 秒 スレッド [KEY=VAL ...]。
名前が v019 で始まれば PyPI の 0.1.9 (<repo>/.v019env) で解く。ENOMOTO_MIP_LOG=1 の標準エラーから、暫定解の行・根の行・最後の 40 行を
logs/問題_名前_s種.log.gz に残し、結果を res_one.jsonl に足す。手元の切り分け用 (同時に複数を回すので時間は目安)。"""
import gzip, json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
p, name, seed, tl, th = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4]), sys.argv[5]
extra = dict(a.split("=", 1) for a in sys.argv[6:])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(extra); env.update(ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
py = str(WT / ".v019env/bin/python") if name.startswith("v019") else sys.executable
t0 = time.time()
with tempfile.TemporaryFile("w+") as err:
    pr = subprocess.run([py, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", th],
                        cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=tl + 600)
    err.seek(0); ls = err.read().splitlines()
try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
KEEP = re.compile(r"new incumbent|root LP|cut loop done|presolve finished|HEUR|heur|restart|SMT|RENS|ALNS|FJ")
keep = [l for l in ls if KEEP.search(l)][:3000]
(D / "logs" / f"{p}_{name}_s{seed}.log.gz").write_bytes(gzip.compress(("\n".join(keep + ["---- 最後の 40 行 ----"] + ls[-40:]) + "\n").encode()))
with open(D / "res_one.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "cfg": name, "seed": seed, "tl": tl, "threads": th, "env": extra, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
print(p, name, seed, res.get("status"), res.get("objective"), res.get("time"), flush=True)
