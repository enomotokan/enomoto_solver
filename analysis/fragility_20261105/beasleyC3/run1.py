"""beasleyC3 の揺れの計測: 1 回回してログを保存し、要約を jsonl に足す。
使い方: python run1.py タグ スレッド数 種 [KEY=VAL ...]"""
import json, os, re, subprocess, sys, time
from pathlib import Path
tag, thr, seed = sys.argv[1], sys.argv[2], sys.argv[3]
extra = dict(a.split("=", 1) for a in sys.argv[4:])
root = Path(__file__).resolve().parents[3]
out = Path(__file__).resolve().parent
prob = os.environ.get("PROB", "beasleyC3")
env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS=thr, ENOMOTO_T_MIP_SEED=seed, **extra)
t0 = time.time()
r = subprocess.run([sys.executable, str(root / "scripts/run_miplib_benchmark.py"), "--worker-ours",
                    str(root / f".miplib_cache/{prob}.mps"), "--time-limit", os.environ.get("TL", "600"), "--threads", thr],
                   env=env, capture_output=True, text=True)
log = r.stderr + r.stdout
name = f"log_{tag}_t{thr}_s{seed}.txt"
(out / name).write_text(log)
res = {}
for line in log.splitlines()[::-1]:
    if line.startswith("{"):
        res = json.loads(line); break
incs = [(float(m.group(1)), m.group(2), float(m.group(3)), m.group(4)) for m in
        re.finditer(r"new incumbent ([\d.e+-]+) at node (\d+) \(([\d.]+)s\) \[(.*?)\]", log)]
rec = dict(tag=tag, threads=int(thr), seed=int(seed), extra=extra, status=res.get("status"), obj=res.get("objective"),
           time=res.get("time"), nodes=res.get("nodes"), wall=round(time.time() - t0, 2), incumbents=incs, log=name)
with open(out / "runs.jsonl", "a") as f:
    f.write(json.dumps(rec, ensure_ascii=False) + "\n")
print(tag, thr, seed, res.get("status"), res.get("objective"), res.get("time"), res.get("nodes"),
      " | ".join(f"{o:g}@{t:.1f}[{s}]" for o, n, t, s in incs if o < 770))
