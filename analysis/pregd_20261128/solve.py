"""PREGD: 1 問を最後まで解く: python solve.py 問題 名札 スレッド 秒 [ENV=値 ...]。種 1。1 行の JSON を solve.jsonl に足す
(状態・目的値・下界・ノード・秒、前処理の後の大きさ (MIP: root LP size の行))。ログは logs/<問題>_<名札>_t<スレッド>.log.gz。"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, tag, th, tl = sys.argv[1], sys.argv[2], sys.argv[3], float(sys.argv[4]); extra = dict(kv.split("=", 1) for kv in sys.argv[5:])
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"}); env.update(extra)
code = f"""
import time, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(WT / '.miplib_cache' / (p + '.mps'))!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, {tl}, None, None, 1, True); t = time.perf_counter() - t
print('RESULT', r['status'], r['objective'], r['best_bound'], r['nodes'], round(t, 2), file=sys.stderr)
"""
out = subprocess.run([sys.executable, "-c", code], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode(errors="replace")
(D / "logs").mkdir(exist_ok=True)
(D / "logs" / f"{p}_{tag}_t{th}.log.gz").write_bytes(gzip.compress(out.encode()))
row = {"problem": p, "tag": tag, "threads": int(th), "tl": tl, "env": extra}
m = re.search(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", out)
if m: row["size"] = [int(x) for x in m.groups()]
m = re.findall(r"^RESULT (.*)$", out, re.M)
if m: row["result"] = m[-1].split()
print(json.dumps(row))
with open(D / "solve.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
