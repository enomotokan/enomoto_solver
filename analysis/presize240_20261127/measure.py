"""PRESIZE240: 240 問で、前処理の後の大きさを当方と HiGHS で測る。
使い方: python measure.py [問題 ...]   (省略時は combo3_20261126/s2/problems.json の 240 問。済んだ問題は飛ばす)
- 当方: 1 スレッド・種 1・ENOMOTO_MIP_LOG=1 で根の LP を解く直前まで回し、"MIP: root LP size" の行 (行・固定でない列・非零) を取る。
  元の大きさは "MIP: presolve" より前の読み込み直後の値ではなく、MPS を highspy で読んだ大きさを使う。上限 600 秒。
- HiGHS 1.15.1: presolve() だけ (1 スレッド、上限 600 秒) の後の大きさ。
出力: res.jsonl に 1 問 1 行、elog/<問題>.log (当方のログ)。キャッシュになかった MPS は測った後に消す (容量の節約)。"""
import json, os, re, subprocess, sys, time, threading
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "res.jsonl"
TL = 600
probs = sys.argv[1:] or json.load(open(WT / "analysis/combo3_20261126/s2/problems.json"))["all"]
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
(D / "elog").mkdir(exist_ok=True)

def ours(mps, p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
    t0 = time.time(); r = {}
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "1"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    killer = threading.Timer(TL + 30, pr.kill); killer.start()
    with open(D / "elog" / f"{p}.log", "w") as f:
        for line in pr.stdout:
            f.write(f"{time.time() - t0:8.2f} {line}")
            m = re.match(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", line)
            if m:
                r.update(rows=int(m.group(1)), cols=int(m.group(2)), nnz=int(m.group(3)), t=round(time.time() - t0, 2)); pr.kill(); break
            if line.startswith("{"):
                try: r["final"] = json.loads(line)
                except ValueError: pass
    killer.cancel(); pr.kill(); pr.wait()
    if "rows" not in r: r["t"] = round(time.time() - t0, 2)
    return r

def highs(mps):
    import highspy
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1); h.setOptionValue("time_limit", float(TL))
    h.readModel(str(mps)); lp = h.getLp()
    orig = {"rows": lp.num_row_, "cols": lp.num_col_, "nnz": len(lp.a_matrix_.index_)}
    t0 = time.time(); st = h.presolve(); t = time.time() - t0
    ps = h.getModelPresolveStatus()
    q = h.getPresolvedLp()
    return orig, {"rows": q.num_row_, "cols": q.num_col_, "nnz": len(q.a_matrix_.index_), "t": round(t, 2), "status": str(ps)}

for p in probs:
    if p in done: continue
    cached = (WT / ".miplib_cache" / f"{p}.mps").exists()
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "error": str(e)[:200]}) + "\n")
        continue
    o = ours(mps, p)
    try: orig, hs = highs(mps)
    except Exception as e: orig, hs = None, {"error": str(e)[:200]}
    rec = {"problem": p, "orig": orig, "ours": o, "highs": hs}
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(p, orig, {k: o.get(k) for k in ("rows", "cols", "nnz", "t")}, hs, flush=True)
    if not cached: mps.unlink(missing_ok=True)
