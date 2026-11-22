"""parspeed の分担を回す: python run.py P K。shards_P.json の K 番目の (問題, 設定, 種, 秒) を順に解き、res_P_sK.jsonl に足す (途中から再開できる)。
analysis/gentop3_20261121/run.py の写しに次を足した:
- 設定は cfg.py (t1・t4・t1c・t4c)。c つきは ENOMOTO_MIP_CUTOFF = 既知の最適値 (scripts/miplib2017-v31.solu の =opt=) - 1e-6 max(1, |最適値|)。
- どの回も ENOMOTO_MIP_LOG=1 で解き、stderr から根と木の分けを読む (行の "split"):
  root_secs = 最後の `TRSTAT: tree start outer X` の X (前処理〜木に入るまで。再スタートがあれば最後の木の始まりまで)、
  restarts = TRSTAT の行の数 - 1、tree_started = TRSTAT の行があるか (なければ根で終わった)、
  finish_secs = `MIP: finish start ... (Xs)` の X。ログは残さない (not_solved・crash の回だけ最後の 60 行を stderr_tail に残す)。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS, threads, has_cutoff  # noqa: E402
SET, k = sys.argv[1], int(sys.argv[2])
mine = json.loads((D / f"shards_{SET}.json").read_text())["shards"][k]
OUT = D / f"res_{SET}_s{k}.jsonl"
OPT = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    f = l.split()
    if len(f) >= 3 and f[0] == "=opt=": OPT[f[1]] = float(f[2])
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
STDERR_TAIL = 60
TR = re.compile(r"^TRSTAT: tree start outer ([0-9.eE+-]+)")
FIN = re.compile(r"^MIP: finish start \S+ \(([0-9.eE+-]+)s\)")
def split_of(lines):
    tr = [float(m.group(1)) for m in map(TR.match, lines) if m]
    fin = [float(m.group(1)) for m in map(FIN.match, lines) if m]
    return {"tree_started": bool(tr), "root_secs": tr[-1] if tr else None, "restarts": max(len(tr) - 1, 0), "finish_secs": fin[-1] if fin else None}
def run(p, cfg, seed, mps, tl):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = threads(cfg); env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
    cut = None
    if has_cutoff(cfg):
        z = OPT[p]; cut = z - 1e-6 * max(1.0, abs(z)); env["ENOMOTO_MIP_CUTOFF"] = repr(cut)
    t0 = time.time(); peak = 0.0
    errf = tempfile.TemporaryFile(mode="w+", encoding="utf-8", errors="replace")
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", threads(cfg)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=errf, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); break
    out, _ = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    errf.seek(0); lines = errf.read().splitlines(); errf.close()
    sp = split_of(lines)
    if str(res.get("status")) == "not_solved" or str(res.get("status")).startswith("crash"):
        res["stderr_tail"] = lines[-STDERR_TAIL:]
    return res, sp, cut, round(peak), round(time.time() - t0, 1)
for p, cfg, seed, tl in mine:
    if (p, cfg, seed) in done: continue
    for attempt in range(5):
        try: mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e: err = e; time.sleep(30)  # noqa: BLE001
    else:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    res, sp, cut, peak, wall = run(p, cfg, seed, mps, tl)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "cutoff": cut, "opt": OPT.get(p), "result": res, "split": sp, "max_rss_mb": peak, "wall": wall,
                            "shard": k, "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), res.get("nodes"), sp["root_secs"], flush=True)
