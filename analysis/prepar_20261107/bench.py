"""既定と試作 (ENOMOTO_MIP_PREPAR) を解き比べる (analysis/prepar_20261107.md)。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|問題,...} 設定,... 種,...
設定: def (環境変数なし) / ple など (ENOMOTO_MIP_PREPAR の値)。行は benchlib.load で読める形 (cfg, rep = 種)。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
if P == "p58":
    from probs import PROBS  # noqa: E402
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
            env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": s, "ENOMOTO_MIP_DEBUG_PRETIME": "1"})
            if c != "def":
                env["ENOMOTO_MIP_PREPAR"] = c
            t0 = time.time()
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                                    cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
                out, err = pr.stdout, pr.stderr
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                err = ""
                res = {"status": "crash", "error": str(e)[-200:]}
            pre = re.search(r"^Presolve: .*\(([\d.]+)s\)", err, re.M)
            r = {"problem": p, "cfg": c, "rep": int(s), "result": res, "wall": round(time.time() - t0, 1),
                 "presolve_s": float(pre.group(1)) if pre else None,
                 "presolve_line": pre.group(0) if pre else None}
            pt = [l for l in err.splitlines() if l.startswith("PRETIME ")]
            if pt:
                r["pretime"] = pt[0]
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), r["presolve_s"], flush=True)
