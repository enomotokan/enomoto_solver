"""CLQPRB の g (既定の形のプロービングの後に含意の形でやり直す) の 2 周目の手間を測る (PRESOLVE-PAR の続き、2026-11-19)。
形: def (既定 1dgrs) / 1dgr / 1dr / 1drs (ENOMOTO_MIP_CLQPRB)。1 スレッド・種 1。
mode=pre: ENOMOTO_MIP_DEBUG_PRETIME=stop で根の前で止め、前処理の秒と PRETIME・CLQPRB/probing の行を読む。
mode=full: 上限 TL 秒で最後まで解く (状態・目的・秒)。ログは logs/<mode>_<問題>_<形>.log.gz。
使い方: python run.py {pre|full} TL OUT.jsonl [問題 ...]  (OUT にある (問題, 形) は飛ばす)。スレッドの数は環境変数 TH (既定 1)。
TH が 1 でなければログの名前に _t<TH> を付ける"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "1dgr": {"ENOMOTO_MIP_CLQPRB": "1dgr"}, "1dr": {"ENOMOTO_MIP_CLQPRB": "1dr"}, "1drs": {"ENOMOTO_MIP_CLQPRB": "1drs"}}
mode, TL, OUT = sys.argv[1], sys.argv[2], Path(sys.argv[3])
TH = os.environ.get("TH", "1")
SFX = "" if TH == "1" else f"_t{TH}"
PROBS = sys.argv[4:] or ["neos8", "mzzv42z", "mzzv11", "neos-3216931-puriri"]
done = set()
if OUT.exists():
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in OUT.read_text().splitlines() if l.strip()}
for p in PROBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, extra in CFGS.items():
        if (p, c) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": TH, "ENOMOTO_T_MIP_SEED": "1"})
        if mode == "pre":
            env["ENOMOTO_MIP_DEBUG_PRETIME"] = "stop"
        env.update(extra)
        t0 = time.time()
        cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", TH]
        try:
            pr = subprocess.run(cmd, cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
            out, err, rc = pr.stdout, pr.stderr, pr.returncode
        except subprocess.TimeoutExpired as e:
            out, err, rc = "", (e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr or "")), "timeout"
        with gzip.open(D / "logs" / f"{mode}_{p}_{c}{SFX}.log.gz", "wt") as f:
            f.write(err)
        r = {"problem": p, "cfg": c, "mode": mode, "threads": int(TH), "wall": round(time.time() - t0, 2), "rc": rc}
        for l in err.splitlines():
            if l.startswith("PRETIME "):
                r["pretime"] = l.strip()
                r["presolve"] = float(l.split()[1].split("=")[1])
        r["clqprb"] = [l.strip() for l in err.splitlines() if "CLQPRB" in l][:6]
        r["probing"] = [l.strip()[:200] for l in err.splitlines() if l.startswith("MIP: probing:") and "probed" in l][:8]
        try:
            res = json.loads(out.strip().splitlines()[-1])
            r["result"] = {k: res.get(k) for k in ("status", "objective", "bound", "time", "nodes")}
        except Exception:
            r["result"] = None
        with OUT.open("a") as f:
            f.write(json.dumps(r) + "\n")
        print(p, c, r.get("presolve"), r["result"] and r["result"].get("time"), r["wall"], flush=True)
