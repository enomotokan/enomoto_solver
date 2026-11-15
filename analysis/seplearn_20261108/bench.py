"""SEPLEARN (analysis/seplearn_20261108.md) の計測。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python bench.py OUT.jsonl TL {p58|tgt|zh|問題,...} 設定,... 種,...
設定の名前と環境変数は CFG。行は benchlib.load で読める形 (cfg, rep = 種)。stderr の SEPLEARN の行 (と MIP: の要約の数行) は
logs/<OUT>_<問題>_<設定>_s<種>.txt に残す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
ZH = "enlight_hard hypothyroid-k1 mzzv42z comp07-2idx physiciansched6-2 wachplan neos-860300 n2seq36q ns1830653".split()
TGT = ZH + ["neos-3627168-kasai", "csched008"]
CFG = {
    "def": {},
    "stat": {"ENOMOTO_MIP_SEPSTAT": "1"},
    "nozh": {"ENOMOTO_MIP_NO_ZEROHALF": "1"},
    "nozhstat": {"ENOMOTO_MIP_NO_ZEROHALF": "1", "ENOMOTO_MIP_SEPSTAT": "1"},
    "sl": {"ENOMOTO_MIP_SEPLEARN": "1"},
}
for k, v in list(CFG.items()):
    pass
EXTRA = os.environ.get("SL_EXTRA_CFG")  # 例: 'sl2=ENOMOTO_MIP_SEPLEARN:1,ENOMOTO_T_MIP_SEPLEARN_SKIP:0.2'
if EXTRA:
    for item in EXTRA.split(";"):
        name, kv = item.split("=", 1)
        CFG[name] = dict(x.split(":", 1) for x in kv.split(","))
OUT, TL, P, cfgs, seeds = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split(","), sys.argv[5].split(",")
if P == "p58":
    from probs import PROBS  # noqa: E402
elif P == "tgt":
    PROBS = TGT
elif P == "zh":
    PROBS = ZH
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
            keep = [l for l in err.splitlines() if l.startswith("SEPLEARN") or "node diag" in l or "root bound" in l.lower() or "cut loop" in l]
            (D / "logs" / f"{OUT.stem}_{p}_{c}_s{s}.txt").write_text("\n".join(keep) + "\n")
            r = {"problem": p, "cfg": c, "rep": int(s), "result": res, "wall": round(time.time() - t0, 1)}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), res.get("nodes"), flush=True)
