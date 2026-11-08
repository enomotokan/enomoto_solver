"""ノード数の調べ (analysis/nodecnt_20261107.md) の ENOMOTO の計測。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|tgt|問題,...} 設定,... 種,...
設定の名前と環境変数は CFG。行は benchlib.load で読める形 (cfg, rep = 種)。stderr は logs/<OUT>_<問題>_<設定>_s<種>.err.gz に残す (p58 では残さない)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
TGT = "neos-3627168-kasai neos-957323 mzzv11 csched008 unitcal_7 sct2 tr12-30".split()
CFG = {
    "def": {},
    "nc": {"ENOMOTO_MIP_NODECNT": "1"},
    "t1": {"ENOMOTO_MIP_THREADS": "1"},
    "t1nc": {"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_NODECNT": "1"},
    "nds": {"ENOMOTO_MIP_SB_NO_DEGEN_SKIP": "1"},
    "ncnds": {"ENOMOTO_MIP_NODECNT": "1", "ENOMOTO_MIP_SB_NO_DEGEN_SKIP": "1"},
}
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
if P == "p58":
    from probs import PROBS  # noqa: E402
elif P == "tgt":
    PROBS = TGT
else:
    PROBS = P.split(",")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for s in seeds:
    for i, p in enumerate(PROBS):
        order = cfgs if i % 2 == 0 else cfgs[::-1]
        for c in order:
            if (p, c, int(s)) in done:
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": s})
            env.update(CFG[c])
            if os.environ.get("NC_TREELOG"):
                env["ENOMOTO_MIP_TREE_LOG"] = "1"
            t0 = time.time()
            err = ""
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                                    cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
                out, err = pr.stdout, pr.stderr
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            if P != "p58":
                with gzip.open(D / "logs" / f"{OUT.stem}_{p}_{c}_s{s}.err.gz", "wt") as f:
                    f.write(err)
            r = {"problem": p, "cfg": c, "rep": int(s), "result": res, "wall": round(time.time() - t0, 1)}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), res.get("nodes"), flush=True)
