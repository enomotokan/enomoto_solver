"""当方の根の下界の推移: python ours_root.py 問題 [名札] [ENV=値 ...]  (OURS_MPS=パス なら別の MPS、例えば HiGHS の前処理後の問題を解く)
1 スレッド・種 1・ノード上限 1 (根だけ)・600 秒、ENOMOTO_MIP_LOG=1 と ENOMOTO_MIP_SEPSTAT=1。ログは logs/ours_<問題>_<名札>.log.gz、
1 行の JSON を ours.jsonl に足す (根の LP・カットのラウンドごとの LP 値・根の下界・時間)。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
p = sys.argv[1]; tag = sys.argv[2] if len(sys.argv) > 2 else "def"; extra = dict(kv.split("=", 1) for kv in sys.argv[3:])
mps = Path(os.environ["OURS_MPS"]) if os.environ.get("OURS_MPS") else WT / ".miplib_cache" / f"{p}.mps"
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_SEPSTAT": "1"}); env.update(extra)
NL = int(os.environ.get("OURS_NODES", "1")); TL = float(os.environ.get("OURS_TL", "600"))
code = f"""
import time, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, {TL}, None, {NL}, 1, True); t = time.perf_counter() - t
print('RESULT', r['status'], r['objective'], r['best_bound'], r['nodes'], r.get('root_bound'), round(t, 2), file=sys.stderr)
"""
t0 = time.time()
out = subprocess.run([sys.executable, "-c", code], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode(errors="replace")
(D / "logs").mkdir(exist_ok=True)
(D / "logs" / f"ours_{p}_{tag}.log.gz").write_bytes(gzip.compress(out.encode()))
row = {"problem": p, "tag": tag, "env": extra, "wall": round(time.time() - t0, 1)}
m = re.search(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", out)
if m: row["root_lp"] = [float(m.group(1)), int(m.group(2)), float(m.group(3))]
row["rounds"] = [float(x) for x in re.findall(r"MIP: cut round \d+: .*?obj (\S+?)[,) ]", out)][:200]
m = re.findall(r"^RESULT (.*)$", out, re.M)
if m: row["result"] = m[-1].split()
print(json.dumps(row)[:600])
with open(D / "ours.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
