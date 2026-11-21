"""根の打ち切り値の診断の 1 回: python one.py PROB SEED TAG THREADS [ENOMOTO_X=V ...]
ログ (ENOMOTO_MIP_LOG=1) を logs/PROB_TAG_tTHREADS_sSEED.log に、結果を res.jsonl に足す。CUTOFF=opt は最適値 + 1e-6·max(1,|opt|) を渡す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
TL = float(os.environ.get("RC_TL", "600"))


def solu():
    ref = {}
    for l in open(WT / "scripts/miplib2017-v31.solu"):
        t = l.split()
        if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = float(t[2])
    return ref


def worker(mps, thr):
    import run_miplib_benchmark as rb
    from enomoto_solver import benchmark_highs as bh
    m, _, _, _ = rb._read_our_model_compat(bh, Path(mps))
    t = time.perf_counter(); r = m.solve(None, False, TL, None, None, int(thr)); t = time.perf_counter() - t
    print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r["best_bound"], "root_bound": r.get("root_bound"),
                      "nodes": r["nodes"], "time": t, "lp_iterations": r.get("lp_iterations")}))


def one(prob, seed, tag, thr, args):
    import run_miplib_benchmark as rb
    mps = rb.fetch(prob, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    opt = solu().get(prob)
    for a in args:
        k, v = a.split("=", 1)
        if k == "CUTOFF":
            v = repr(opt + 1e-6 * max(1.0, abs(opt))); k = "ENOMOTO_MIP_CUTOFF"
        env[k] = v
    env["ENOMOTO_MIP_THREADS"] = str(thr); env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
    log = D / "logs" / f"{prob}_{tag}_t{thr}_s{seed}.log"
    t0 = time.time()
    with open(log, "w") as lf:
        pr = subprocess.run([sys.executable, __file__, "_w", str(mps), str(thr)], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True, timeout=TL + 300)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    row = {"problem": prob, "seed": seed, "tag": tag, "threads": thr, "args": args, "tl": TL, "result": res, "wall": round(time.time() - t0, 1),
           "opt": opt, "at": time.strftime("%m-%d %H:%M:%S")}
    with open(D / "res.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
    print(prob, seed, tag, thr, res.get("status"), res.get("nodes"), round(res.get("time") or 0, 1), res.get("root_bound"), flush=True)


if __name__ == "__main__":
    if sys.argv[1] == "_w": worker(sys.argv[2], sys.argv[3])
    else: one(sys.argv[1], int(sys.argv[2]), sys.argv[3], int(sys.argv[4]), sys.argv[5:])
