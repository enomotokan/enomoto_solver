"""ENOMOTO_MIP_ROOT_OBJ_PROBE_HULL の効きの確かめ: python analysis/objprobehull_20261124/run.py 出力.jsonl 問題 [問題 ...] [--seeds 1,2,3] [--tl 120] [--mincands0] [--cfgs def,hull,hull2,og]
各問題を 1 スレッド・種ごとに既定 (def) と試作 (hull) で解き、結果と ENOMOTO_MIP_LOG のうち根の打ち切り値つきプロービング・再スタートの行を残す。
済んだ (問題, 設定, 種) は飛ばす。問題ファイルは .miplib_cache (scripts/run_miplib_benchmark.py の fetch)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
args = sys.argv[1:]
seeds, tl = [1, 2, 3], 120.0
if "--seeds" in args:
    i = args.index("--seeds"); seeds = [int(s) for s in args[i + 1].split(",")]; del args[i:i + 2]
if "--tl" in args:
    i = args.index("--tl"); tl = float(args[i + 1]); del args[i:i + 2]
# --mincands0: 両方の設定で既存のつまみ ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MIN_CANDS=0 を置く (P の 58 問は既定の見切りで試し自体が走らないため)
base = {}
if "--mincands0" in args:
    args.remove("--mincands0"); base = {"ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MIN_CANDS": "0"}
out, probs = Path(args[0]), args[1:]
done = set()
if out.exists():
    for l in open(out):
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"], json.dumps(r.get("base", {}))))
# 設定: def (既定)、hull (=1、境界だけ)、hull2 (=2、関係も)、og (ROOT_OBJPROP_GLOBAL=1)。--cfgs def,hull2 で選ぶ (既定は def,hull)
ALL = {"def": {}, "hull": {"ENOMOTO_MIP_ROOT_OBJ_PROBE_HULL": "1"}, "hull2": {"ENOMOTO_MIP_ROOT_OBJ_PROBE_HULL": "2"},
       "og": {"ENOMOTO_MIP_ROOT_OBJPROP_GLOBAL": "1"}}
names = ["def", "hull"]
if "--cfgs" in args:
    i = args.index("--cfgs"); names = args[i + 1].split(","); del args[i:i + 2]
    out, probs = Path(args[0]), args[1:]
CFG = {c: {**base, **ALL[c]} for c in names}
KEEP = ("root objective probing", "ROOT_OBJPROP_GLOBAL", "ROOT_OBJ_PROBE_HULL=2", "restart check", "Restart", "restart:", "root LP", "root bound")
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for s in seeds:
        for c, extra in CFG.items():
            if (p, c, s, json.dumps(base)) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": str(s), "ENOMOTO_MIP_LOG": "1", **extra})
            t = time.time()
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "1"],
                                    cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=tl + 300)
                lines = [l for l in pr.stdout.splitlines() if l.startswith("{")]
                res = json.loads(lines[-1]) if lines else {"status": "crash"}
                log = [l for l in pr.stderr.splitlines() if any(k in l for k in KEEP)]
            except subprocess.TimeoutExpired:
                res, log = {"status": "killed"}, []
            rec = {"problem": p, "base": base, "cfg": c, "seed": s, "tl": tl, "wall": time.time() - t, "result": res, "log": log}
            with open(out, "a") as f:
                f.write(json.dumps(rec, ensure_ascii=False) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
