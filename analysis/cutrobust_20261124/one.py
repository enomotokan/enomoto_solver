"""1 回解く: python one.py 問題 名札 [ENV=値 ...]
既定は 1 スレッド・種 1・60 秒 (CR_TH・CR_SEED・CR_TL で変える)、ENOMOTO_MIP_LOG=1。ログは logs/<問題>_<名札>.log.gz、
1 行の JSON を res.jsonl に足す (根の LP、カットのラウンドごとの LP 値、根の下界、ノード、時間)。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
p = sys.argv[1]; tag = sys.argv[2]; extra = dict(kv.split("=", 1) for kv in sys.argv[3:])
mps = WT / ".miplib_cache" / f"{p}.mps"
TH = os.environ.get("CR_TH", "1"); SEED = os.environ.get("CR_SEED", "1"); TL = float(os.environ.get("CR_TL", "60"))
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": TH, "ENOMOTO_T_MIP_SEED": SEED, "ENOMOTO_MIP_LOG": "1"}); env.update(extra)
code = f"""
import time, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, {TL}, None, None, {TH}, True); t = time.perf_counter() - t
print('RESULT', r['status'], r['objective'], r['best_bound'], r['nodes'], r.get('root_bound'), round(t, 2), file=sys.stderr)
"""
t0 = time.time()
out = subprocess.run([sys.executable, "-c", code], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode(errors="replace")
(D / "logs").mkdir(exist_ok=True)
(D / "logs" / f"{p}_{tag}_th{TH}_s{SEED}.log.gz").write_bytes(gzip.compress(out.encode()))
row = {"problem": p, "tag": tag, "env": extra, "th": TH, "seed": SEED, "tl": TL, "wall": round(time.time() - t0, 1)}
m = re.search(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", out)
if m: row["root_lp"] = [float(m.group(1)), int(m.group(2)), float(m.group(3))]
row["rounds"] = [float(x) for x in re.findall(r"MIP: cut round \d+: .*?obj (\S+?)[,) ]", out)][:100]
m = re.findall(r"^RESULT (.*)$", out, re.M)
if m: row["result"] = m[-1].split()
else: row["tail"] = out[-600:]
print(json.dumps(row)[:700])
with open(D / "res.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
