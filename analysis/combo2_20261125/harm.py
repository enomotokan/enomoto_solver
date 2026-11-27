"""combo2 の害の切り分け (analysis/comboharm_20261124/run.py の写し、部品は c2 の 9 つ): python analysis/combo2_20261125/harm.py OUT.jsonl 問題 スレッド数 CFG[,CFG...] [--tl 秒] [--par N] [--seed S]
設定: def・c2a・-X (c2a から X を抜く)・+X (既定に X を足す)。X は cfg.py の C2 の短い名前 (NAMES)。
同じ (問題, 設定, スレッド, 種) の行があれば飛ばす。stderr の末尾 (4000 字) も残す (ENOMOTO_MIP_LOG はつけない)。"""
import json, os, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import C2 as COMBO  # noqa: E402
NAMES = {"nra": "ENOMOTO_MIP_NODEROWS_ADAPT", "enum": "ENOMOTO_MIP_ENUM_ADAPT", "heur": "ENOMOTO_MIP_HEUR_ADAPT", "fp3ls": "ENOMOTO_MIP_FP_STAGE3_LS",
         "lmip": "ENOMOTO_MIP_LMIP_SCORE_TICKS", "bfrt": "ENOMOTO_MIP_NODELP_BFRT_GROUP", "tie": "ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER",
         "cutu": "ENOMOTO_MIP_CUTOFF_USER_TREE", "ppi": "ENOMOTO_MIP_PROOFPROP"}
# c2c を土台にする形 (親の依頼、blend2): `c2c` = c2 + SB_PROBEBD=1、`c2c-X` = c2c から X を抜く (X に sbpb も使える)、`+sbpb` = 既定に SB_PROBEBD=1
SBPB = {"ENOMOTO_MIP_SB_PROBEBD": "1"}
assert set(NAMES.values()) == set(COMBO)


def cfg_env(c):
    if c == "def": return {}
    if c == "c2a": return dict(COMBO)
    if c == "c2c": return dict(COMBO, **SBPB)
    if c.startswith("c2c-"):
        x = c[4:]
        return {k: v for k, v in dict(COMBO, **SBPB).items() if k != ("ENOMOTO_MIP_SB_PROBEBD" if x == "sbpb" else NAMES[x])}
    if c.startswith("-"): return {k: v for k, v in COMBO.items() if k != NAMES[c[1:]]}
    if c.startswith("+"): return {k: v for x in c[1:].split("+") for k, v in (SBPB.items() if x == "sbpb" else [(NAMES[x], COMBO[NAMES[x]])])}
    raise ValueError(c)


args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name); v = args[i + 1]; del args[i:i + 2]; return v
    return default
TL = float(opt("--tl", "600")); PAR = int(opt("--par", "1")); SEED = int(opt("--seed", "1"))
OUT = Path(args[0]); prob = args[1]; th = args[2]; cfgs = args[3].split(",")
mps = rb.fetch(prob, WT / ".miplib_cache")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["threads"], r["seed"]))


def one(c):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(cfg_env(c)); env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = str(SEED)
    env.update(dict(a.split("=", 1) for a in os.environ.get("CH_EXTRA", "").split() if a))
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", th],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 900)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "rc": pr.returncode}  # noqa: BLE001
    return {"problem": prob, "cfg": c, "threads": th, "seed": SEED, "tl": TL, "extra": os.environ.get("CH_EXTRA", ""), "result": res,
            "stderr_tail": pr.stderr[-4000:], "wall": round(time.time() - t0, 1)}


todo = [c for c in cfgs if (prob, c, th, SEED) not in done]
with ThreadPoolExecutor(PAR) as ex:
    for r in ex.map(one, todo):
        with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
        res = r["result"]; print(prob, th, r["cfg"], res.get("status"), res.get("time"), res.get("nodes"), res.get("objective"), flush=True)
