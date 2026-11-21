"""§8 の確かめを回す: python chk8.py N5 (neos5 × {def, sft0} × 種 1〜10 × 各 2 回、60 秒) か python chk8.py SK (sorrell3・kakapo × {def, sft0} × 種 1、600 秒)。
どれも 4 スレッド、ENOMOTO_MIP_LOG=1 と ENOMOTO_MIP_DEBUG_DUMP_SUBINF=wlogs/dump/<問題>_<設定>_s<種>_r<回> (前処理で実行不能になった sub-MIP を書き出す)。
結果は res_<組>.jsonl に足す (途中から再開できる)。行の形は wrun.py と同じで、回 (rep) と書き出した sub-MIP の数 (dumps) を足す。ログは wlogs/<組>/ (コミットしない)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
from wrun import count_submips  # noqa: E402
SET = sys.argv[1]
if SET == "N5":
    JOBS = [("neos5", c, s, r, 60) for s in range(1, 11) for r in (1, 2) for c in ("def", "sft0")]
else:
    JOBS = [(p, c, 1, 1, 600) for p in ("sorrell3", "neos-5107597-kakapo") for c in ("def", "sft0")]
OUT = D / f"res_{SET}.jsonl"
LD = D / "wlogs" / SET; LD.mkdir(parents=True, exist_ok=True)
DD = D / "wlogs" / "dump"; DD.mkdir(parents=True, exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"], r["rep"]))
for p, cfg, seed, rep, tl in JOBS:
    if (p, cfg, seed, rep) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    tag = f"{p}_{cfg}_s{seed}_r{rep}"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
    env["ENOMOTO_MIP_DEBUG_DUMP_SUBINF"] = str(DD / tag)
    log = LD / f"{tag}.log"
    t0 = time.time()
    with open(log, "w") as fe:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True, timeout=tl + 300)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    txt = log.read_text(errors="replace")
    sub = count_submips(txt)
    with open(log, "rb") as fi, gzip.open(str(log) + ".gz", "wb") as fo: fo.write(fi.read())
    log.unlink()
    nd = len(list(DD.glob(f"{tag}_*.mps")))
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "rep": rep, "tl": tl, "result": res, "submip": sub, "dumps": nd,
                            "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, rep, res.get("status"), res.get("time"), sub["inf0"], sub["inf0_by"], nd, flush=True)
