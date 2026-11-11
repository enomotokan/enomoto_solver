"""IPMRC (analysis/ipmrc_20261111.md) の計測 (XOLAZY の台本を写したもの)。ソルバーは同時に 1 つ (4 スレッド)。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|m30|問題,...} 設定,... 種,... [回の番号 (既定 0)]
設定の名前と環境変数は CFG (どの設定も診断 ENOMOTO_MIP_XOLAZY_LOG を付ける。振る舞いは変えない)。行は benchlib.load で読める形
(cfg, rep = 種)。stderr の XOLAZY の行は logs/<OUT>_<問題>_<設定>_s<種>_r<回>.txt に残す。
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
    # 診断: 二段解法が勝っても内点法の点を待つ (採る結果は変えない)
    "wait": {"ENOMOTO_MIP_IPMRC_WAIT": "1"},
}
# Mittelmann で根の LP が重い問題 (analysis/rootlp3_20261111/m47scr.jsonl で根の LP の和が 0.1 秒以上の 25 問 + 186 問から LP の大きい 5 問)
M30 = ["satellites2-60-fs", "mzzv11", "neos-3216931-puriri", "neos-933966", "mzzv42z", "uccase12", "physiciansched6-2",
       "neos-4722843-widden", "mushroom-best", "seymour1", "n2seq36q", "cod105", "neos-827175", "unitcal_7", "mc11",
       "neos-873061", "net12", "neos-957323", "neos-1445765", "hypothyroid-k1", "neos-960392", "supportcase33", "neos-860300",
       "neos-2978193-inde", "h80x6320d", "rail01", "momentum1", "satellites2-40", "thor50dday", "trento1"]
# HiGHS だけが解く 13 問 (analysis/mitt600_20261103.md の res9)
H13 = ["csched008", "mzzv11", "neos-1456979", "neos-3216931-puriri", "neos-3627168-kasai", "neos-4722843-widden", "neos-873061", "net12",
       "rococoC10-001000", "satellites2-60-fs", "sct2", "tr12-30", "uccase12"]
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
REP = int(sys.argv[6]) if len(sys.argv) > 6 else 0
if not OUT.is_absolute():
    OUT = D / OUT
(D / "logs").mkdir(exist_ok=True)
if P == "p58":
    from probs import PROBS  # noqa: E402
elif P == "m30":
    PROBS = M30
elif P == "m40":
    PROBS = M30 + [q for q in H13 if q not in M30]
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
            env.update({"ENOMOTO_MIP_XOLAZY_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": s})
            env.update(CFG[c])
            t0 = time.time()
            tag = f"{OUT.stem}_{p}_{c}_s{s}_r{REP}"
            fo, fe = D / "logs" / f"{tag}.out", D / "logs" / f"{tag}.err"
            rss = None
            try:
                with fo.open("w") as so, fe.open("w") as se:
                    wrap = "import resource,subprocess,sys; subprocess.run(sys.argv[1:]); print('MAXRSS', resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss, file=sys.stderr)"
                    subprocess.run([sys.executable, "-c", wrap, sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL,
                                    "--threads", "4"], cwd=WT, env=env, stdout=so, stderr=se, timeout=float(TL) + 300)
                out = fo.read_text()
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            err = fe.read_text() if fe.exists() else ""
            rss = next((int(l.split()[1]) for l in err.splitlines() if l.startswith("MAXRSS ")), None)
            keep = [l for l in err.splitlines() if l.startswith("XOLAZY")]
            (D / "logs" / f"{tag}.txt").write_text("\n".join(keep) + "\n")
            fo.unlink(missing_ok=True); fe.unlink(missing_ok=True)
            r = {"problem": p, "cfg": c, "rep": int(s), "run": REP, "result": res, "maxrss_kb": rss, "wall": round(time.time() - t0, 1),
                 "xolazy": [json.loads(l[7:]) for l in keep if l.startswith("XOLAZY ")]}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, REP, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), rss, len(r["xolazy"]), flush=True)
