"""HiGHS・SCIP に負けている問題 (picked.json の 48 問) の測り直し (analysis/losses_20261123、analysis/holdout_20261123/run.py の写し)。
1 回ずつ別プロセスで、同時に 1 本だけ解く。結果は 1 回 1 行で足し、同じ (問題, 設定, 種, 上限秒) の行があれば飛ばす (途中から再開できる)。

使い方 (リポジトリの根か、このフォルダから):
  python analysis/losses_20261123/run.py K                # 分担 K (shards.json の K 番目、plan.py で作る) → res_sK.jsonl
  python analysis/losses_20261123/run.py --tl 10 --problems neos8,air03 --out /tmp/x.jsonl   # 分担を使わずに選ぶ (確かめ用)
各行に、読み込んだ当方の .so の sha1 (`ours_so_sha1`) を残す (dist/build.txt の so_sha1 と同じなら、同じ wheel で測った)。

設定 (CFGS):
- ours_t1 / ours_t4: 当方の既定 (ENOMOTO_ で始まる環境変数は全部外す)、1 / 4 スレッド (ENOMOTO_MIP_THREADS)。種は ENOMOTO_T_MIP_SEED。
- scip: pyscipopt の SCIP (既定で逐次)。limits/gap = 0、種は randomization/randomseedshift (analysis/vsscip_20261117 と同じ)。
- highs_t1 / highs_t4: highspy の HiGHS、threads 1 / 4。mip_rel_gap = 0 (analysis/vshighs_20261117 と同じ)。種は random_seed
  (vshighs は種を渡していなかった。種 1 つなら影響は小さい)。
どれも scripts/run_miplib_benchmark.py の worker (--worker-ours / --worker-scip / --worker-highs) で解き、時間は読み込みを除いた求解の時間。
"""
import argparse, json, os, subprocess, sys, time
from importlib import metadata
from pathlib import Path

D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402

CFGS = {
    "ours_t1": ("ours", 1),
    "ours_t4": ("ours", 4),
    "scip": ("scip", 1),
    "highs_t1": ("highs", 1),
    "highs_t4": ("highs", 4),
}


def versions():
    v = {}
    for pkg in ("enomoto-solver", "highspy", "pyscipopt"):
        try:
            v[pkg] = metadata.version(pkg)
        except metadata.PackageNotFoundError:
            v[pkg] = None
    return v


def ours_so_sha1():
    """子プロセスと同じ読み込み方で当方の拡張 (.so / .pyd) を探し、その sha1 を返す (入れた wheel の確かめ)"""
    import hashlib
    r = subprocess.run([sys.executable, "-c", "import enomoto_solver._core as c; print(c.__file__)"], cwd=WT / "scripts",
                       env={k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}, stdout=subprocess.PIPE, text=True)
    p = r.stdout.strip()
    return hashlib.sha1(open(p, "rb").read()).hexdigest() if p and os.path.exists(p) else None


def rss_mb(pid):
    """子プロセスの常駐メモリ (MB)。psutil があればそれで (Windows・macOS も)、なければ Linux の /proc から。読めなければ 0"""
    try:
        import psutil
        return psutil.Process(pid).memory_info().rss / 2**20
    except ImportError:
        pass
    except Exception:  # noqa: BLE001
        return 0.0
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"):
                return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0


def run_one(cfg, seed, mps, tl):
    kind, th = CFGS[cfg]
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env["ENOMOTO_MIP_THREADS"] = str(th)
    env["ENOMOTO_T_MIP_SEED"] = str(seed)
    env["ENOMOTO_BENCH_SCIP_SEED"] = str(seed)
    if kind == "highs":
        env["ENOMOTO_BENCH_HIGHS_OPT"] = f"random_seed={seed}"
    t0 = time.time(); peak = 0.0
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", str(th)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    killed = False
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300:
            pr.kill(); killed = True; break
    out, _ = pr.communicate()
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "killed" if killed else "crash", "returncode": pr.returncode}
    return res, round(peak), round(time.time() - t0, 1)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("shard", nargs="?", type=int, help="分担の番号 (shards.json)。与えると --problems・--seeds・--cfgs・--tl は使わない")
    ap.add_argument("--tl", type=float, default=600, help="1 回の時間の上限 (秒、既定 600)")
    ap.add_argument("--seeds", default="1", help="種 (カンマ区切り、既定 1)")
    ap.add_argument("--cfgs", default=",".join(CFGS), help=f"設定 (カンマ区切り、既定は全部: {','.join(CFGS)})")
    ap.add_argument("--problems", default="", help="問題 (カンマ区切りか、@ファイル (1 行 1 問))。既定は picked.json の全部")
    ap.add_argument("--out", default="", help="結果のファイル (既定: 分担 K なら res_sK.jsonl、そうでなければ res_x.jsonl)")
    ap.add_argument("--cache", default=str(WT / ".miplib_cache"), help="MPS を置くフォルダ (既定 リポジトリの .miplib_cache)")
    a = ap.parse_args()
    allp = sorted(json.loads((D / "picked.json").read_text(encoding="utf-8")))
    if a.problems.startswith("@"):
        probs = [l.strip() for l in open(a.problems[1:], encoding="utf-8") if l.strip() and not l.startswith("#")]
    elif a.problems:
        probs = [x.strip() for x in a.problems.split(",") if x.strip()]
    else:
        probs = allp
    cfgs = [c.strip() for c in a.cfgs.split(",") if c.strip()]
    bad = [c for c in cfgs if c not in CFGS]
    if bad:
        sys.exit(f"知らない設定: {bad} (使えるのは {list(CFGS)})")
    seeds = [int(s) for s in a.seeds.split(",") if s.strip()]
    out = Path(a.out or (D / (f"res_s{a.shard}.jsonl" if a.shard is not None else "res_x.jsonl")))
    done = set()
    if out.exists():
        for l in out.read_text(encoding="utf-8").splitlines():
            if l.strip():
                r = json.loads(l)
                if r["result"].get("status") not in ("fetch_error", "crash"):  # 取得の失敗・crash は次の回にやり直す
                    done.add((r["problem"], r["cfg"], r["seed"], float(r["tl"])))
    ver = versions()
    ver["ours_so_sha1"] = ours_so_sha1()
    jobs = []
    if a.shard is not None:
        jobs = [(p, c, s, tl) for p, c, s, tl in json.loads((D / "shards.json").read_text(encoding="utf-8"))["shards"][a.shard]]
    else:
        for i, p in enumerate(probs):
            for s in seeds:
                k = (i + s) % len(cfgs)  # 同じ (問題, 種) の設定は続けて解き、順は (問題, 種) ごとに回して入れ替える
                jobs += [(p, c, s, a.tl) for c in cfgs[k:] + cfgs[:k]]
    todo = [j for j in jobs if j not in done]
    print(f"{len(jobs)} 回のうち {len(jobs) - len(todo)} 回は済み、残り {len(todo)} 回 → {out}  版: {ver}", flush=True)
    for n, (p, cfg, seed, tl) in enumerate(todo, 1):
        err = None
        for attempt in range(5):
            try:
                mps = rb.fetch(p, Path(a.cache)); break
            except Exception as e:  # noqa: BLE001
                err = e; time.sleep(30)
        else:
            with open(out, "a", encoding="utf-8") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
            continue
        res, peak, wall = run_one(cfg, seed, mps, tl)
        with open(out, "a", encoding="utf-8") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "max_rss_mb": peak, "wall": wall,
                                "versions": ver, "platform": sys.platform, "nproc": os.cpu_count(), "at": time.strftime("%Y-%m-%d %H:%M:%S")}) + "\n")
        print(f"[{n}/{len(todo)}] {p} {cfg} 種 {seed}: {res.get('status')} {res.get('objective')} {round(res.get('time') or 0, 1)} 秒, {peak} MB", flush=True)


if __name__ == "__main__":
    main()
