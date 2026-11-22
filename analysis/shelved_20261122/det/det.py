"""設定が本当に計算を変えているかを、1 スレッド・決定的に見る (analysis/shelved_20261122/README.md の「悪化の切り分け」)。
def・ov・ipmxm を同じ種 (1)・1 スレッド・ノード数の上限 (ENOMOTO_MIP_NODE_LIMIT、既定 200) で回し、時刻を除いたログの指紋を比べる。
python det.py 問題... >> det.jsonl。ログは logs/PROB_CFG.log.gz。
比べるもの: 根の LP の値と反復、カットのラウンドごとの目的値・LP 反復、外側の根の止まり方と手間 (CUTSTOP の tk_loop・tk_lp)、
終わりの状態・目的値・下界・ノード数、内点法・クロスオーバーの行の数 (ipmxm が働いたか)"""
import gzip, hashlib, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D.parent))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
NL = os.environ.get("DET_NODE_LIMIT", "200")
TL = os.environ.get("DET_TL", "1800")
TIME = re.compile(r"\(?[0-9.]+s\)?|[0-9.]+ ?(?:secs?|seconds|ms)\b|t_loop\":[0-9.]+|\"t\":[0-9.]+|at [0-9.]+s")
for p in sys.argv[1:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c in ("def", "ov", "ipmxm"):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CFGS[c])
        env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_CUTSTOP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_NODE_LIMIT": NL})
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=int(TL) + 600)
        err = pr.stderr
        with gzip.open(D / "logs" / f"{p}_{c}.log.gz", "wt") as f: f.write(err)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        rootlp = re.findall(r"MIP: root LP (\S+) \((\d+) iters", err)
        rounds = [(a, b) for a, b in re.findall(r"MIP: cut round \d+: .*?LP (\d+) iters\).*?obj (\S+)", err)]
        cs = [json.loads(m) for m in re.findall(r"CUTSTOP (\{.*\})", err)]
        cs = [(x["reason"], x["rounds"], x.get("tk_loop"), x.get("tk_lp"), x["obj1"]) for x in cs if not x.get("bg")]
        ipm = len(re.findall(r"(?i)interior|crossover|IPM", err))
        # 時刻を除いた「MIP:」の行の並びの指紋 (道が同じかの目安。スレッドの競走のない 1 スレッドで、時刻だけが違うはず)
        lines = [TIME.sub("", l) for l in err.splitlines() if l.startswith("MIP:") and "LMBOOST" not in l]
        fp = hashlib.sha1("\n".join(lines).encode()).hexdigest()[:12]
        row = {"problem": p, "cfg": c, "status": res.get("status"), "objective": res.get("objective"), "bound": res.get("bound"), "nodes": res.get("nodes"),
               "time": res.get("time"), "root_bound": res.get("root_bound"), "rootlp": rootlp[:3], "rounds": rounds, "cutstop": cs, "ipm_lines": ipm,
               "fp": fp, "nlines": len(lines)}
        with open(D / "det.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(p, c, row["status"], row["nodes"], row["root_bound"], row["bound"], rootlp[:1], len(rounds), cs[-1:] , ipm, fp, flush=True)
