"""(analysis/nrashare_20261126/run.py の写し) c2c の 1 スレッドの遅れの切り分け (analysis/c2c1t_20261126): python analysis/c2c1t_20261126/run.py OUT.jsonl 問題[,問題...] 種[,種...] スレッド 秒 設定[,設定...]
設定: c2c (analysis/combo2_20261125/cfg.py の C2C)・c2cs (c2c + ENOMOTO_MIP_NRA_SHARE=1)・c2cp (c2c の nra を r0k23p)・c2cps (c2cp + 共有)・def。1 本ずつ順に解く (4 スレッドは 4 コアを使う)。
同じ (問題, 設定, スレッド, 種) の行があれば飛ばす。止めても同じコマンドで続きから回る。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "combo2_20261125"))
sys.path.insert(0, str(WT / "scripts"))
from cfg import C2C  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "c2c": dict(C2C), "c2cs": dict(C2C, ENOMOTO_MIP_NRA_SHARE="1"),
        # 親の追加: nra の採用候補 r0k23p (`p` は φ の補正) の上でも、共有なし・ありを比べる
        "c2cp": dict(C2C, ENOMOTO_MIP_NODEROWS_ADAPT="r0k23p"), "c2cps": dict(C2C, ENOMOTO_MIP_NODEROWS_ADAPT="r0k23p", ENOMOTO_MIP_NRA_SHARE="1")}
# neos-631710 の直し (README 7 節): c2c の ENUM_ADAPT を 1st にした形 (`t`: FJ_P3 の前処理に時計を置く)
CFGS["c2ct"] = dict(C2C, ENOMOTO_MIP_ENUM_ADAPT="1st")
CFGS["c2cpt"] = dict(C2C, ENOMOTO_MIP_ENUM_ADAPT="1st", ENOMOTO_MIP_NODEROWS_ADAPT="r0k23p")
# 親の追加 (neos-631710 の 4 スレッドの遅れの切り分け): c2c から部品を 1 つ抜いた形 "-名前"
PARTS = {"nra": "ENOMOTO_MIP_NODEROWS_ADAPT", "enum": "ENOMOTO_MIP_ENUM_ADAPT", "heur": "ENOMOTO_MIP_HEUR_ADAPT", "fpls": "ENOMOTO_MIP_FP_STAGE3_LS",
         "lmip": "ENOMOTO_MIP_LMIP_SCORE_TICKS", "bfrt": "ENOMOTO_MIP_NODELP_BFRT_GROUP", "tie": "ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER",
         "cutu": "ENOMOTO_MIP_CUTOFF_USER_TREE", "ppi": "ENOMOTO_MIP_PROOFPROP", "sbpb": "ENOMOTO_MIP_SB_PROBEBD"}
assert set(PARTS.values()) == set(C2C), set(C2C) ^ set(PARTS.values())
for _k, _v in PARTS.items():
    CFGS["-" + _k] = {k: v for k, v in C2C.items() if k != _v}
    CFGS["+" + _k] = {_v: C2C[_v]}
# c2c1t (2026-11-26、親の依頼): nra は r0k23p。c2cp から部品を 1 つ抜いた形 "p-名前"・1 つだけ足した形 "p+名前"
C2CP = dict(C2C, ENOMOTO_MIP_NODEROWS_ADAPT="r0k23p")
for _k, _v in PARTS.items():
    CFGS["p-" + _k] = {k: v for k, v in C2CP.items() if k != _v}
    CFGS["p+" + _k] = {_v: C2CP[_v]}
# 直しの試作 (README 3 節): c2cp + 強分枝の子の伝播の伸びを擬似費用に入れない (`ENOMOTO_MIP_SB_PROP_PSKIP`)
CFGS["c2cpk"] = dict(C2CP, ENOMOTO_MIP_SB_PROP_PSKIP="1")
CFGS["p+sbpbk"] = {"ENOMOTO_MIP_SB_PROBEBD": "1", "ENOMOTO_MIP_SB_PROP_PSKIP": "1"}
# 観測を捨てずに入れ直す形 (README 7 節、親の依頼): `l` = 双対の下界 (案 2)、`t` = 伝播なしで解き直す (案 1)
CFGS["c2cpl"] = dict(C2CP, ENOMOTO_MIP_SB_PROP_PSKIP="l")
CFGS["c2cpt"] = dict(C2CP, ENOMOTO_MIP_SB_PROP_PSKIP="t")
OUT = Path(sys.argv[1]); probs = sys.argv[2].split(","); seeds = [int(x) for x in sys.argv[3].split(",")]; th = sys.argv[4]; tl = float(sys.argv[5]); cfgs = sys.argv[6].split(",")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l)
        if r["result"].get("status") not in ("crash",):
            done.add((r["problem"], r["cfg"], r["threads"], r["seed"]))
for seed in seeds:
    for p in probs:
        mps = rb.fetch(p, WT / ".miplib_cache")
        for c in cfgs:
            if (p, c, th, seed) in done: continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CFGS[c]); env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = str(seed)
            t0 = time.time()
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", th],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 900)
            try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash", "rc": pr.returncode, "err": pr.stderr[-1500:]}  # noqa: BLE001
            with open(OUT, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "threads": th, "seed": seed, "tl": tl, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
            print(p, c, th, seed, res.get("status"), res.get("time"), res.get("nodes"), res.get("objective"), flush=True)
