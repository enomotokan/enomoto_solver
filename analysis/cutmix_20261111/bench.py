"""CUTMIX (analysis/cutmix_20261111.md) の計測。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|m47|問題,...} 設定,... 種,...
設定の名前と環境変数は CFG (全部 `ENOMOTO_MIP_LOG=1` で回し、stat の付く設定は `ENOMOTO_MIP_SEPSTAT=1` も)。
行は benchlib.load で読める形 (cfg, rep = 種)。stderr の CUTMIX・SEPLEARN の行と MIP: の要約の数行は
logs/<OUT>_<問題>_<設定>_s<種>.txt に残す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
M47 = ("cod105 csched008 drayage-100-23 drayage-25-23 fiball h80x6320d hypothyroid-k1 mc11 mcsched mushroom-best mzzv11 mzzv42z n2seq36q "
       "neos-1445765 neos-1456979 neos-2978193-inde neos-3216931-puriri neos-3627168-kasai neos-4722843-widden neos-827175 neos-860300 "
       "neos-873061 neos-911970 neos-933966 neos-957323 neos-960392 neos8 net12 ns1116954 ns1830653 p200x1188c pg pg5_34 physiciansched6-2 "
       "rococoC10-001000 roll3000 satellites2-60-fs sct2 seymour1 supportcase33 swath1 tbfp-network timtab1 tr12-30 traininstance6 uccase12 "
       "unitcal_7").split()
CFG = {
    "def": {},
    "stat": {"ENOMOTO_MIP_SEPSTAT": "1"},
    "lcstat": {"ENOMOTO_MIP_SEPSTAT": "1", "ENOMOTO_MIP_ROW_LIFTED_COVER": "1"},
    "zhstat": {"ENOMOTO_MIP_SEPSTAT": "1", "ENOMOTO_MIP_NO_ZEROHALF": "0"},
    "lc": {"ENOMOTO_MIP_ROW_LIFTED_COVER": "1"},
    "zh": {"ENOMOTO_MIP_NO_ZEROHALF": "0"},
}
EXTRA = os.environ.get("CM_EXTRA_CFG")  # 例: 'cm1=ENOMOTO_MIP_CUTMIX:lc'
if EXTRA:
    for item in EXTRA.split(";"):
        name, kv = item.split("=", 1)
        CFG[name] = dict(x.split(":", 1) for x in kv.split(","))
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
if P == "p58":
    from probs import PROBS  # noqa: E402
elif P == "m47":
    PROBS = M47
else:
    PROBS = P.split(",")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
(D / "logs").mkdir(exist_ok=True)
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
            if os.environ.get("CM_PKG"):  # 凍結した拡張のコピー (計測中に作業木を作り直しても混ざらない)
                env["PYTHONPATH"] = os.environ["CM_PKG"]
            t0 = time.time()
            err = ""
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", env["ENOMOTO_MIP_THREADS"]],
                                    cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
                out, err = pr.stdout, pr.stderr
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            keep = [l for l in err.splitlines() if l.startswith("SEPLEARN") or l.startswith("CUTMIX") or "node diag" in l or "root bound" in l.lower()
                    or "cut loop" in l or "clique graph" in l or "clique extension" in l or "restart" in l.lower()[:40]]
            keep = [l for l in keep if l.startswith(("SEPLEARN", "CUTMIX"))][-2000:] + [l for l in keep if not l.startswith(("SEPLEARN", "CUTMIX"))][-200:]
            (D / "logs" / f"{OUT.stem}_{p}_{c}_s{s}.txt").write_text("\n".join(keep) + "\n")
            r = {"problem": p, "cfg": c, "rep": int(s), "result": res, "wall": round(time.time() - t0, 1)}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), res.get("nodes"), flush=True)
