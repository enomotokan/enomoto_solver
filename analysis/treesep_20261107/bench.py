"""木の中の分離の計測 (analysis/treesep_20261107.md)。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|tgt|問題,...} 設定,... 種,...
設定の名前と環境変数は CFG。行は benchlib.load で読める形 (cfg, rep = 種)。stderr の診断の行は logs/ に残す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
TGT = "sct2 neos-3627168-kasai uccase12 unitcal_7 tr12-30 neos-957323 mzzv11 neos-1456979 rococoC10-001000 fiball csched008".split()
CFG = {
    "def": {},
    "nots": {"ENOMOTO_MIP_NO_TREE_SEP": "1"},
    "every": {"ENOMOTO_MIP_NO_TREE_SEP": "1", "ENOMOTO_MIP_NODE_CUTS": "1"},
    "frac10": {"ENOMOTO_MIP_NO_TREE_SEP": "1", "ENOMOTO_T_MIP_NODE_CUT_TIME_FRAC": "0.1"},
}
for k in "a b c d ab abd ad bd abcd e ae".split():
    CFG[k] = {"ENOMOTO_MIP_TREESEP2": k}
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
if P == "p58":
    from probs import PROBS  # noqa: E402
elif P == "tgt":
    PROBS = TGT
else:
    PROBS = P.split(",")
DIAG = os.environ.get("TSB_DIAG", "")
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
            if DIAG:
                env.update({"ENOMOTO_MIP_TREESEP_DIAG": DIAG, "ENOMOTO_MIP_NCA_DIAG": "1"})
            env.update(CFG[c])
            t0 = time.time()
            err = ""
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                                    cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
                out, err = pr.stdout, pr.stderr
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            keep = [l for l in err.splitlines() if l.startswith(("TSDIAG", "MIP: node diag", "MIP: adaptive", "MIP: tree search"))
                    or (l.startswith("MIP: ") and " nodes " in l and " lb " in l)]
            (D / "logs" / f"{OUT.stem}_{p}_{c}_s{s}.log").write_text("\n".join(keep) + "\n")
            r = {"problem": p, "cfg": c, "rep": int(s), "result": res, "wall": round(time.time() - t0, 1)}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), flush=True)
