"""既定 (def) と試作 (hnf = ENOMOTO_MIP_HNFPRE=1、hnfb = =bounds、lat = ENOMOTO_MIP_HNFPRE_LAT=1、luold = ENOMOTO_T_LU_DENSE_SING_TOL=0) を 4 スレッドで解く (2026-11-07)。1 回ずつ順に (ソルバーの計測は同時に 1 つ)。
使い方: python run.py out.jsonl T cfgs(カンマ区切り) seeds(カンマ区切り) 問題...  (problem に "@p58" と書くと 58 問)。
既に out にある (問題, 設定, 回) は飛ばす (止まったら同じ命令で続きから回る)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
import hprobs
CFG = {"def": {}, "hnf": {"ENOMOTO_MIP_HNFPRE": "1"}, "hnfb": {"ENOMOTO_MIP_HNFPRE": "bounds"},
       # 書き直し (hnflat) と、稠密 LU の特異判定の修正の前 (許容 0 = 厳密な 0 だけを特異とする以前の判定)
       "lat": {"ENOMOTO_MIP_HNFPRE_LAT": "1"}, "luold": {"ENOMOTO_T_LU_DENSE_SING_TOL": "0"}}
out = Path(sys.argv[1]); T = float(sys.argv[2]); cfgs = sys.argv[3].split(","); seeds = [int(s) for s in sys.argv[4].split(",")]
probs = []
for a in sys.argv[5:]:
    probs += hprobs.SETS[a[1:]] if a.startswith("@") else [a]
done = set()
if out.exists():
    for l in out.open():
        try:
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
        except Exception:
            pass
for s in seeds:
    for p in probs:
        for c in cfgs:
            if (p, c, s) in done:
                continue
            env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(s), ENOMOTO_MIP_HNFPRE_QUIET="1", **CFG[c])
            env.pop("ENOMOTO_MIP_HNFPRE", None) if c == "def" else None
            mps = WT / ".miplib_cache" / f"{p}.mps"
            t0 = time.time()
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(T), "--threads", "4"],
                                capture_output=True, text=True, env=env, timeout=T * 3 + 120)
            try:
                res = json.loads(pr.stdout.strip().splitlines()[-1])
            except Exception:
                res = {"status": "error", "err": pr.stderr[-500:]}
            rec = {"problem": p, "cfg": c, "rep": s, "result": res, "wall": round(time.time() - t0, 2)}
            with out.open("a") as f:
                f.write(json.dumps(rec) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
