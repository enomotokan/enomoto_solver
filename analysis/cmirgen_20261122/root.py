"""根だけ (1 スレッド・NODE_LIMIT=1・再スタートなし・種 1): python root.py 並べる数 [設定 ...]。問題 PROBS × 設定 (cfg.py) を解き、
CUTSTOP (最初の根のループの理由・ラウンド・下界) と全体の結果を root.jsonl に足す (済んだ組は飛ばす)。"""
import json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
PROBS = os.environ.get("CMIRGEN_PROBS", "neos-1582420 unitcal_7 mc11 p200x1188c beasleyC3 neos-911970 exp-1-500-5-5 timtab1 qiu nw04").split()
OUT = D / "root.jsonl"
def one(job):
    p, c = job
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_NO_RESTART": "1", "ENOMOTO_MIP_CUTSTOP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
    env.update(CFGS[c])
    r = subprocess.run([sys.executable, str(R / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(R / ".miplib_cache" / f"{p}.mps"), "--time-limit", "900", "--threads", "1"],
                       cwd=R, env=env, capture_output=True, text=True)
    cs = [json.loads(l.split("CUTSTOP", 1)[1]) for l in r.stderr.splitlines() if l.startswith("CUTSTOP {")]
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    row = {"problem": p, "cfg": c, "cut": {k: cs[0][k] for k in ("reason", "rounds", "obj0", "obj1")} if cs else None, "result": res}
    with open(OUT, "a") as f:
        f.write(json.dumps(row) + "\n")
    print(c, p, row["cut"], round(res["time"], 1), flush=True)
if __name__ == "__main__":
    nj = int(sys.argv[1]) if len(sys.argv) > 1 else 3
    cf = sys.argv[2:] or list(CFGS)
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(OUT)} if OUT.exists() else set()
    with ThreadPoolExecutor(nj) as ex:
        list(ex.map(one, [(p, c) for p in PROBS for c in cf if (p, c) not in done]))
