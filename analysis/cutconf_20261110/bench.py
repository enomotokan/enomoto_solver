"""cut-based の衝突解析 (analysis/cutconf_20261110.md) の ENOMOTO の計測。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|cand|m47|問題,...} 設定,... 種,...
設定の名前と環境変数は CFG。行は benchlib.load で読める形 (cfg, rep = 種) に、cutconf の統計の行 (`cc`) を足す。
stderr は logs/<OUT>_<問題>_<設定>_s<種>.err.gz に残す。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
CAND = "mas76 mas74 noswot timtab1 neos-957323 tr12-30 neos-3627168-kasai".split()
CFG = {
    "def": {},
    "cc": {"ENOMOTO_MIP_CUTCONF": "1"},
    "ccS": {"ENOMOTO_MIP_CUTCONF": "S"},
    "ccm": {"ENOMOTO_MIP_CUTCONF": "m"},
    "ccmp": {"ENOMOTO_MIP_CUTCONF": "mp"},
    "ccr": {"ENOMOTO_MIP_CUTCONF": "r"},
    "ccd": {"ENOMOTO_MIP_CUTCONF": "d"},
    "ccx": {"ENOMOTO_MIP_CUTCONF": "x"},
    "ccrx": {"ENOMOTO_MIP_CUTCONF": "rx"},
    "ccb": {"ENOMOTO_MIP_CUTCONF": "b"},
}
for k, v in list(CFG.items()):
    if k.startswith("cc") and os.environ.get("CC_EXTRA"):
        # CC_EXTRA="名前=値,..." を試作の設定に足す
        CFG[k] = dict(v, **dict(x.split("=", 1) for x in os.environ["CC_EXTRA"].split(",")))
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
if P == "p58":
    from probs import PROBS  # noqa: E402
elif P == "cand":
    PROBS = CAND
elif P == "m47":
    PROBS = json.load(open(WT / "analysis/mitt600_20261103/shards8.json"))["problems"]
else:
    PROBS = P.split(",")
(D / "logs").mkdir(exist_ok=True)
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
                                    cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) * 2 + 300)
                out, err = pr.stdout, pr.stderr
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            with gzip.open(D / "logs" / f"{OUT.stem}_{p}_{c}_s{s}.err.gz", "wt") as f:
                f.write(err)
            cc = [l for l in err.splitlines() if l.startswith("MIP: cutconf")]
            dbg = sum(1 for l in err.splitlines() if "MIP_DEBUG_SOL: cutconf" in l)
            r = {"problem": p, "cfg": c, "rep": int(s), "result": res, "wall": round(time.time() - t0, 1), "cc": cc[-1] if cc else "", "dbg": dbg}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), res.get("nodes"), flush=True)
