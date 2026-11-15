"""HIGHSLOSS2 (c): 分離器を切れる HiGHS 1.15.1 (hsep.patch を当てて pip install した highspy) で 1 問を解く。
python hsep.py 問題 切る文字 時間 [スレッド]   切る文字: i=implied bound, c=clique, t=tableau, p=path (aggregation), m=mod-k, '-'=何も切らない。
HiGHS の環境変数 HSEP_OFF・HSEP_LOG (根の各ラウンドの分離器ごとのプールに入った本数と LP 値を stderr に出す) を立てて、
scripts/run_miplib_benchmark.py の --worker-highs で回す。ログは logs/hsep_<問題>_<文字>.log.gz、結果は hsep.jsonl に 1 行。
python は環境変数 HSEP_PY (改造版の highspy の入った venv の python)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
p, off, tl = sys.argv[1], sys.argv[2], float(sys.argv[3])
tag = os.environ.get("HSEP_TAG", "")
th = sys.argv[4] if len(sys.argv) > 4 else "4"
py = os.environ.get("HSEP_PY", sys.executable)
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_BENCH_HIGHS_LOG="1", HSEP_LOG="1", HSEP_OFF="" if off == "-" else off)
t0 = time.time()
r = subprocess.run([py, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-highs", str(WT / ".miplib_cache" / f"{p}.mps"),
                    "--time-limit", str(tl), "--threads", th], cwd=WT, env=env, capture_output=True, text=True)
(D / "logs").mkdir(exist_ok=True)
with gzip.open(D / "logs" / f"hsep_{p}_{off}{tag}.log.gz", "wt") as f:
    f.write(r.stdout + "\n#### stderr\n" + r.stderr)
try:
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "rc": r.returncode}
hs = [l for l in r.stderr.splitlines() if l.startswith("HSEP round")]
with open(D / "hsep.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "off": off, "tl": tl, "threads": th, "result": res, "root_rounds": len(hs),
                        "root_last": hs[-1] if hs else None, "wall": round(time.time() - t0, 1)}) + "\n")
print(p, off, res.get("status"), res.get("objective"), res.get("bound"), res.get("time"), len(hs))
