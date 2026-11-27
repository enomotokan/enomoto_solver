"""PRESIZE240: SCIP 10.0 (pyscipopt) の前処理だけを回し、後の大きさを取る。
使い方: python scip.py [問題 ...]   (省略時は 240 問。済んだ問題は飛ばす)
前処理の後の問題を MPS に書き出し (setppc・knapsack・logicor などは線形の行になる)、highspy で読んで行・列・非零を数える。
1 スレッド、前処理の上限 600 秒。出力: scip.jsonl。キャッシュになかった MPS は消す。"""
import json, sys, time, subprocess
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "scip.jsonl"
probs = sys.argv[1:] or json.load(open(WT / "analysis/combo3_20261126/s2/problems.json"))["all"]
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
CHILD = r'''
import json, sys, time, pyscipopt, highspy
mps, tmp = sys.argv[1], sys.argv[2]
m = pyscipopt.Model(); m.hideOutput(); m.setParam("limits/time", 600.0); m.setParam("lp/threads", 1)
m.readProblem(mps)
t0 = time.time(); m.presolve(); t = time.time() - t0
st = m.getStatus(); r = {"t": round(t, 2), "status": st, "vars": m.getNVars(), "conss": m.getNConss()}
if st not in ("infeasible", "unbounded", "inforunbd", "optimal") and m.getNConss() > 0:
    m.writeProblem(tmp, trans=True)
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(tmp); lp = h.getLp()
    r.update(rows=lp.num_row_, cols=lp.num_col_, nnz=len(lp.a_matrix_.index_))
print(json.dumps(r))
'''
for p in probs:
    if p in done: continue
    cached = (WT / ".miplib_cache" / f"{p}.mps").exists()
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "error": str(e)[:200]}) + "\n")
        continue
    tmp = D / f"_scip_{p}.mps"
    try:
        pr = subprocess.run([sys.executable, "-c", CHILD, str(mps), str(tmp)], capture_output=True, text=True, timeout=900)
        r = json.loads(pr.stdout.strip().splitlines()[-1]) if pr.stdout.strip() else {"error": pr.stderr[-300:]}
    except Exception as e: r = {"error": str(e)[:200]}
    tmp.unlink(missing_ok=True)
    rec = {"problem": p, "scip": r}
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(p, r, flush=True)
    if not cached: mps.unlink(missing_ok=True)
