"""1 問を 4 スレッドで解き、結果と根の LP の様子 (時間・反復・根の LP の値・カットの後の値・SIMPLEX_SCALE の行) を <出力>.jsonl に足す。
使い方: python run.py 設定の名前 問題 時間 回の番号 出力の名前。設定は CFGS (ログは logs/ に gzip で置く)"""
import json, os, subprocess, sys, time, re, gzip
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
CFGS = {
    # base は既定化 (2026-11-07) の前のビルドでの既定。既定化の後のビルドで同じ形を回すには ENOMOTO_MIP_ROOTLP_SCALE=0 を足す
    "base": {},
    "fix": {"ENOMOTO_MIP_ROOTLP_SCALE": "1"},
    "fixm5k": {"ENOMOTO_MIP_ROOTLP_SCALE": "1", "ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_M": "5000"},
    "fixr1e3": {"ENOMOTO_MIP_ROOTLP_SCALE": "1", "ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_RANGE": "1e3"},
    "fixr1e6": {"ENOMOTO_MIP_ROOTLP_SCALE": "1", "ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_RANGE": "1e6"},
}
cfg, p, tl, rep, out = sys.argv[1:6]
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_SIMPLEX_SCALE_LOG="1", **CFGS[cfg])
(D / "logs").mkdir(exist_ok=True)
# 書き途中のログはリポジトリの外に置く (コミット・rebase で書き先が置き換わらないように)。終わったら logs/ に gzip で置く
TMP = Path(os.environ.get("ROOTLP_TMP", "/tmp/rootlp_logs")); TMP.mkdir(exist_ok=True)
log = TMP / f"{out}_{cfg}_{p}_{rep}.log"
t0 = time.time()
pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                     "--time-limit", tl, "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=open(log, "w"), text=True, timeout=float(tl) + 400)
try:
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
txt = log.read_text(errors="replace")
rl = re.search(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", txt)
cl = re.search(r"MIP: cut loop done: .* obj \S+ -> (\S+)", txt)
info = dict(root_lp_s=float(rl[3]) if rl else None, root_iters=int(rl[2]) if rl else None, root_obj=float(rl[1]) if rl else None,
            root_cut_obj=float(cl[1]) if cl else None, scale=[l for l in txt.splitlines() if l.startswith(("SIMPLEX_SCALE:", "ROOTLP_SCALE:"))][:6],
            rescue=("re-solved with the scaled" in txt))
with gzip.open(str(D / "logs" / log.name) + ".gz", "wt") as g: g.write(txt)
log.unlink()
with open(D / f"{out}.jsonl", "a") as f:
    f.write(json.dumps({"problem": p, "cfg": cfg, "rep": int(rep), "result": res, "info": info, "wall": round(time.time() - t0, 1)}) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("best_bound"), info["root_lp_s"], flush=True)
