"""ROOTLP3 (analysis/rootlp3_20261111.md) の計測。ソルバーは同時に 1 つ (4 スレッド)。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|問題,...} 設定,... 種,... [回の番号 (既定 0)]
設定の名前と環境変数は CFG (どの設定も診断 ENOMOTO_MIP_ROOTLP3_LOG を付ける。振る舞いは変えない)。行は benchlib.load で読める形
(cfg, rep = 種)。stderr の ROOTLP3 の行と NB_DIAG の行 (付けたとき) は logs/<OUT>_<問題>_<設定>_s<種>_r<回>.txt に残す。
最大 RSS (KB) は間に挟んだ Python の getrusage(RUSAGE_CHILDREN) で取る。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
CFG = {
    "def": {},
    "s": {"ENOMOTO_MIP_ROOTLP3": "s"},
    "defnb": {"ENOMOTO_MIP_NB_DIAG": "1"},
    "snb": {"ENOMOTO_MIP_ROOTLP3": "s", "ENOMOTO_MIP_NB_DIAG": "1"},
}
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
REP = int(sys.argv[6]) if len(sys.argv) > 6 else 0
if not OUT.is_absolute():
    OUT = D / OUT
if P == "p58":
    from probs import PROBS  # noqa: E402
else:
    PROBS = P.split(",")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"], r.get("run", 0)))
for s in seeds:
    for i, p in enumerate(PROBS):
        order = cfgs if (i + int(s)) % 2 == 0 else cfgs[::-1]
        for c in order:
            if (p, c, int(s), REP) in done:
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_ROOTLP3_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": s})
            env.update(CFG[c])
            t0 = time.time()
            tag = f"{OUT.stem}_{p}_{c}_s{s}_r{REP}"
            fo, fe = D / "logs" / f"{tag}.out", D / "logs" / f"{tag}.err"
            rss = None
            try:
                with fo.open("w") as so, fe.open("w") as se:
                    # 子のプロセスの最大 RSS は、間に挟んだ小さな Python が getrusage(RUSAGE_CHILDREN) で stderr に出す
                    wrap = "import resource,subprocess,sys; subprocess.run(sys.argv[1:]); print('MAXRSS', resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss, file=sys.stderr)"
                    subprocess.run([sys.executable, "-c", wrap, sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL,
                                    "--threads", "4"], cwd=WT, env=env, stdout=so, stderr=se, timeout=float(TL) + 300)
                out = fo.read_text()
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            err = fe.read_text() if fe.exists() else ""
            rss = next((int(l.split()[1]) for l in err.splitlines() if l.startswith("MAXRSS ")), None)
            keep = [l for l in err.splitlines() if l.startswith("ROOTLP3") or l.startswith("NB_DIAG")]
            (D / "logs" / f"{tag}.txt").write_text("\n".join(keep) + "\n")
            fo.unlink(missing_ok=True); fe.unlink(missing_ok=True)
            r = {"problem": p, "cfg": c, "rep": int(s), "run": REP, "result": res, "maxrss_kb": rss, "wall": round(time.time() - t0, 1),
                 "rootlp3": [json.loads(l[8:]) for l in keep if l.startswith("ROOTLP3 ")]}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, REP, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), rss, len(r["rootlp3"]), flush=True)
