"""S6 の試作の害を 58 問 (analysis/cbcimpl_20261029/probs.py) で見る (4 スレッド・60 秒)。
使い方: python bench58.py 設定,... rep   (設定は CFGS の名前。rep は種 ENOMOTO_T_MIP_SEED にも使う。問題ごとに設定の順を入れ替える)
結果は bench58.jsonl に 1 行ずつ足す (済んだものは飛ばす)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
CFGS = {
    "def": {},
    "c0jc": {"ENOMOTO_MIP_HO4_CUT0": "1", "ENOMOTO_MIP_TREE3_JOINCAP": "1"},
    "c0": {"ENOMOTO_MIP_HO4_CUT0": "1"},
    "p3": {"ENOMOTO_MIP_POOL3_BEST": "0.05"},
    "hard2": {"ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_HARD": "2"},
    "tcap": {"ENOMOTO_MIP_S6_CUTLP_TCAP": "10"},
    "jcx": {"ENOMOTO_MIP_TREE3_JOINCAP": "1", "ENOMOTO_MIP_ROBUST_STALL": "rw", "ENOMOTO_MIP_ROBUST_LNSFREQ": "1", "ENOMOTO_MIP_ROBUST_IMPORT": "1", "ENOMOTO_MIP_MEMBQ_LNSGROW": "1"},
}
cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
OUT = D / "bench58.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for i, p in enumerate(PROBS):
    order = cfgs if (i + rep) % 2 == 0 else cfgs[::-1]
    for c in order:
        if (p, c, rep) in done: continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[c])
        t0 = time.time(); peak = 0
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                              cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        while pr.poll() is None:
            try:
                peak = max(peak, next(int(l.split()[1]) // 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:")))
            except (OSError, StopIteration):
                pass
            time.sleep(0.2)
            if time.time() - t0 > 300: pr.kill()
        out = pr.stdout.read()
        try:
            res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception:  # noqa: BLE001
            res = {"status": "crash", "returncode": pr.returncode}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "wall": round(time.time() - t0, 1), "max_rss_mb": peak}) + "\n")
        print(p, c, rep, res.get("status"), round(res.get("time") or 0, 1), flush=True)
