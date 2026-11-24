"""列挙とかけ直しの適応形 (8 節) の確かめ: python adapt_run.py 問題 スレッド 名札 [ENV=値 ...]
最後まで解く (種 1・60 秒、ADAPT_TL で変える)。ENOMOTO_MIP_LOG=1 のログを logs/adapt_<問題>_t<スレッド>_<名札>.log.gz に、
1 行の JSON (状態・目的値・下界・ノード・根の下界・秒・前処理の秒・列挙の行・適応の f) を adapt.jsonl に足す。1 本ずつ順に回す。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
p, th, tag = sys.argv[1], sys.argv[2], sys.argv[3]; extra = dict(kv.split("=", 1) for kv in sys.argv[4:])
mps = rb.fetch(p, WT / ".miplib_cache")
TL = float(os.environ.get("ADAPT_TL", "60"))
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"}); env.update(extra)
code = f"""
import time, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, {TL}, None, None, {int(th)}, True); t = time.perf_counter() - t
print('RESULT', r['status'], r['objective'], r['best_bound'], r['nodes'], r.get('root_bound'), round(t, 2), file=sys.stderr)
"""
t0 = time.time()
out = subprocess.run([sys.executable, "-c", code], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode(errors="replace")
(D / "logs").mkdir(exist_ok=True)
(D / "logs" / f"adapt_{p}_t{th}_{tag}.log.gz").write_bytes(gzip.compress(out.encode()))
row = {"problem": p, "threads": int(th), "tag": tag, "env": extra, "wall": round(time.time() - t0, 1)}
m = re.search(r"^Presolve: \d+ rows, \d+ columns -> \d+ rows, \d+ columns \(([\d.]+)s\)", out, re.M)
if m: row["pre_secs"] = float(m.group(1))
row["enum"] = [[g[0]] + [float(x) for x in g[1:]] for g in re.findall(r"MIP: enumeration presolve \(([a-z ]+)\): (\d+) rows, (\d+) solutions, (\d+) fixed, (\d+) bounds tightened, (\d+) equivalent pairs, (\d+) new conflict pairs.*?\(([\d.]+)s\)", out)]
row["enum_loop"] = len(re.findall(r"MIP: ENUM_WHERE=loop: round", out))
row["prr_tight"] = len(re.findall(r"PRR_TIGHT", out))
row["adapt"] = re.findall(r"MIP: ENUM_ADAPT: (.*)", out)
m = re.findall(r"^RESULT (.*)$", out, re.M)
if m: row["result"] = m[-1].split()
print(json.dumps(row)[:700])
with open(D / "adapt.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
