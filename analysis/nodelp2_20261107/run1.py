"""ノードの LP の計数・試作の 1 回の計測 (2026-11-06 の node_lp_20261106/run1.py の写し。rep を種 ENOMOTO_T_MIP_SEED に使う)。
1 問を 4 スレッド・TL 秒で解き、結果と stderr の `MIP: nodelp` の行を <出力>.jsonl に 1 行で書く (stderr 全体は logs/<問題>_<設定>_<rep>.log)。
使い方: python run1.py 出力.jsonl 設定 問題 秒 rep
設定は CFGS の名前 (diag: 既定 + 計数 ENOMOTO_MIP_NODELP_DIAG、plain: 環境変数なし、ほかは試作 + 計数)。
"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
DG = {"ENOMOTO_MIP_NODELP_DIAG": "1"}
CFGS = {
    "diag": dict(DG),
    "plain": {},
    "h2t": dict(DG, ENOMOTO_SIMPLEX_HARRIS2="t"),
    "h2r": dict(DG, ENOMOTO_SIMPLEX_HARRIS2="r"),
    "pert": dict(DG, ENOMOTO_NODE_PERTURB="1"),
    # 試作 ENOMOTO_MIP_NODELP_PERTURB (ノードの LP だけ費用の摂動 p/P と前向きの Harris h、g は重い LP のときだけ)
    "ph": dict(DG, ENOMOTO_MIP_NODELP_PERTURB="ph"),
    "phg": dict(DG, ENOMOTO_MIP_NODELP_PERTURB="phg"),
    "Ph": dict(DG, ENOMOTO_MIP_NODELP_PERTURB="Ph"),
    "ch": dict(DG, ENOMOTO_MIP_CHILD_HIGHS="1"),
    "bnt": dict(DG, ENOMOTO_MIP_BASIS_NB="t"),
    "phb": dict(DG, ENOMOTO_MIP_NODELP_PERTURB="ph", ENOMOTO_MIP_BASIS_NB="t"),
    "plainphgb": {"ENOMOTO_MIP_NODELP_PERTURB": "phg", "ENOMOTO_MIP_BASIS_NB": "t"},
    "plainphb": {"ENOMOTO_MIP_NODELP_PERTURB": "ph", "ENOMOTO_MIP_BASIS_NB": "t"},
    "chph": dict(DG, ENOMOTO_MIP_CHILD_HIGHS="1", ENOMOTO_MIP_NODELP_PERTURB="ph"),
    "plainph": {"ENOMOTO_MIP_NODELP_PERTURB": "ph"},
    "plainphg": {"ENOMOTO_MIP_NODELP_PERTURB": "phg"},
}
for k, v in list(os.environ.items()):
    # 追加の設定: NODELP_CFG_<名前>="K=V,K=V"
    if k.startswith("NODELP_CFG_"):
        CFGS[k[len("NODELP_CFG_"):].lower()] = dict(DG, **dict(x.split("=", 1) for x in v.split(",")))
out, cfg, p, tl, rep = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[cfg])
t0 = time.time()
(D / "logs").mkdir(exist_ok=True)
try:
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 2 + 300)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    err = r.stderr
except Exception as e:  # noqa: BLE001
    res = {"status": "crash", "error": str(e)[:300]}
    err = ""
(D / "logs" / f"{p}_{cfg}_{rep}.log").write_text(err)
nl = [l for l in err.splitlines() if l.startswith("MIP: nodelp")]
with open(out, "a") as f:
    f.write(json.dumps({"problem": p, "cfg": cfg, "rep": rep, "tl": tl, "wall": round(time.time() - t0, 1), "result": res, "nodelp": nl}) + "\n")
print(p, cfg, rep, res.get("status"), round(res.get("time") or 0, 1), res.get("nodes"), res.get("objective"), res.get("bound"), flush=True)
