"""primalgap の直しの広い計測の分担を回す: python wrun.py SET K (SET は P (MIPLIB の 58 問 × 種 1〜3、60 秒) か M (Mittelmann 186 問 × 種 1、600 秒))。
shards_SET.json の K 番目の (問題, 設定, 種, 秒) を順に解き、res_SET_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ。
行の形は analysis/rootgap_20261121/run.py と同じで、ENOMOTO_MIP_LOG=1 のログから数えたサブ MIP の結果 (`submip`、[`count_submips`]) を足す。
ログは wlogs/SET/<問題>_<設定>_s<種>.log.gz (コミットしない)。設定は cfg.py、どれも 4 スレッド。"""
import gzip, json, os, re, subprocess, sys, time
from collections import Counter
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
PAR = re.compile(r"MIP:   parallel sub-MIP \((.*?)\): status (\w+), nodes (\d+), objective (\S+), ([\d.]+)s")
SEQ = re.compile(r"MIP:   sub-MIP: \d+ free columns of \d+, status (\w+), nodes (\d+), objective (\S+?),? ([\d.]+)s")
TREE = re.compile(r"^MIP: +[\d.]+s nodes ")
def count_submips(txt):
    """サブ MIP の結果の行を数える: 全部 (n)・状態 Infeasible でノード 0 (inf0)・その秒の和 (inf0_s)・inf0 の出どころ (並列は名前、主のスレッドは "seq")。
    root_* は木の最初の経過の行 (`MIP: <秒>s nodes`) より前だけ"""
    out = {"n": 0, "inf0": 0, "inf0_s": 0.0, "root_n": 0, "root_inf0": 0, "root_inf0_s": 0.0, "inf0_by": Counter()}
    root = True
    for l in txt.splitlines():
        if root and TREE.match(l): root = False
        m = PAR.match(l)
        if m: lab, st, nd, secs = m.group(1), m.group(2), int(m.group(3)), float(m.group(5))
        else:
            m = SEQ.match(l)
            if not m: continue
            lab, st, nd, secs = "seq", m.group(1), int(m.group(2)), float(m.group(4))
        out["n"] += 1; out["root_n"] += root
        if st == "Infeasible" and nd == 0:
            out["inf0"] += 1; out["inf0_s"] += secs; out["inf0_by"][lab] += 1
            if root: out["root_inf0"] += 1; out["root_inf0_s"] += secs
    out["inf0_s"] = round(out["inf0_s"], 2); out["root_inf0_s"] = round(out["root_inf0_s"], 2); out["inf0_by"] = dict(out["inf0_by"])
    return out
if __name__ == "__main__":
    SET, k = sys.argv[1], int(sys.argv[2])
    mine = json.loads((D / f"shards_{SET}.json").read_text())["shards"][k]
    OUT = D / f"res_{SET}_s{k}.jsonl"
    LD = D / "wlogs" / SET; LD.mkdir(parents=True, exist_ok=True)
    done = set()
    if OUT.exists():
        for l in OUT.read_text().splitlines():
            if l.strip(): r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
    def rss_mb(pid):
        try:
            for l in open(f"/proc/{pid}/status"):
                if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
        except OSError:
            pass
        return 0.0
    def run(p, cfg, seed, mps, tl):
        env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
        env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
        log = LD / f"{p}_{cfg}_s{seed}.log"
        t0 = time.time(); peak = 0.0
        with open(log, "w") as fe:
            pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                                  cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
            while pr.poll() is None:
                peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
                if time.time() - t0 > tl + 300: pr.kill(); break
            out, _ = pr.communicate()
        try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        txt = log.read_text(errors="replace")
        sub = count_submips(txt)
        with open(log, "rb") as fi, gzip.open(str(log) + ".gz", "wb") as fo: fo.write(fi.read())
        log.unlink()
        return res, sub, round(peak), round(time.time() - t0, 1)
    for p, cfg, seed, tl in mine:
        if (p, cfg, seed) in done: continue
        for attempt in range(5):
            try: mps = rb.fetch(p, WT / ".miplib_cache"); break
            except Exception as e: err = e; time.sleep(30)  # noqa: BLE001
        else:
            with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
            continue
        res, sub, peak, wall = run(p, cfg, seed, mps, tl)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "submip": sub, "max_rss_mb": peak, "wall": wall, "shard": k,
                                "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, cfg, seed, res.get("status"), res.get("time"), sub["inf0"], peak, flush=True)
