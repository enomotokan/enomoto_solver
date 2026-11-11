"""ROOTSTOP (analysis/rootstop_20261111.md) の計測。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|問題,...} 設定,... 種,...
設定の名前と環境変数は CFG (ROOTSTOP_EXTRA_CFG で足せる。例 'r01=ENOMOTO_MIP_ROOTSTOP:r,ENOMOTO_T_MIP_ROOTSTOP_RATE:0.01')。
行は (problem, cfg, rep = 種, result, wall, root = rootinfo.py の出力)。標準エラーの全体は LOGDIR (既定はこの下の logs/) に gz で残す。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
CFG = {
    "def": {},
    "r": {"ENOMOTO_MIP_ROOTSTOP": "r"},
    "b": {"ENOMOTO_MIP_ROOTSTOP": "b"},
    "rb": {"ENOMOTO_MIP_ROOTSTOP": "rb"},
}
EXTRA = os.environ.get("ROOTSTOP_EXTRA_CFG")
if EXTRA:
    for item in EXTRA.split(";"):
        name, kv = item.split("=", 1)
        CFG[name] = dict(x.split(":", 1) for x in kv.split(","))
LOGDIR = Path(os.environ.get("ROOTSTOP_LOGDIR", D / "logs"))
LOGDIR.mkdir(parents=True, exist_ok=True)
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
            env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": s})
            env.update(CFG[c])
            t0 = time.time()
            err = ""
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", env["ENOMOTO_MIP_THREADS"]],
                                    cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
                out, err = pr.stdout, pr.stderr
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            lg = LOGDIR / f"{OUT.stem}_{p}_{c}_s{s}.log.gz"
            with gzip.open(lg, "wt") as f:
                f.write(err)
            tmp = LOGDIR / "_tmp.log"
            tmp.write_text(err)
            try:
                root = json.loads(subprocess.run([sys.executable, str(D / "rootinfo.py"), str(tmp)], capture_output=True, text=True).stdout)
            except Exception:  # noqa: BLE001
                root = None
            extra = {"rs_r": err.count("ROOTSTOP r:"), "rs_b_skip": err.count("background cuts skipped"), "rs_b_start": err.count("restart not expected")}
            r = {"problem": p, "cfg": c, "rep": int(s), "result": res, "wall": round(time.time() - t0, 1), "root": root, **extra}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), res.get("nodes"), root and root.get("tree_abs"), root and root.get("restarts"), flush=True)
