"""P (MIPLIB 58 問) と M (Mittelmann の残り 145 問) で、1 スレッドの根まで (前処理の LP による固定・根の LP) にクロスオーバーの
Megiddo の押し出しを通り、そこで基底の核の引き金 (Fill など。照合・x_B の残差以外) の再分解が起きる問題を数える。
既定 (ENOMOTO_XO_BUMP_LU_RATIO なし)、1 スレッド・60 秒・ENOMOTO_DEBUG_CROSSOVER=1、2 問並べて回す。
python xoscan.py → xoscan.jsonl (途中から再開できる)"""
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
mitt = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in PROBS] + [("M", p) for p in mitt if p not in PROBS]
OUT = D / "xoscan.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
PAT = re.compile(r"megiddo prof: total=([\d.]+)s .*refactor=([\d.]+) \((\d+)x, (\d+) by verify, (\d+) by x_B residual\)")
DPAT = re.compile(r"dual megiddo: .* refactors=(\d+) t=([\d.]+)s")

def one(sp):
    s, p = sp
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        return {"set": s, "problem": p, "error": str(e)[:200]}
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_DEBUG_CROSSOVER": "1"})
    try:
        pr = subprocess.run(["nice", "-n", "19", sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=200)
        out = pr.stderr
    except subprocess.TimeoutExpired as e:
        out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    pushes = [tuple(float(x) for x in m.groups()) for m in PAT.finditer(out)]
    duals = [tuple(float(x) for x in m.groups()) for m in DPAT.finditer(out)]
    # 基底の核の引き金の再分解 = 全体 - 照合 - x_B の残差
    kern = sum(int(a[2] - a[3] - a[4]) for a in pushes)
    return {"set": s, "problem": p, "pushes": len(pushes), "push_secs": round(sum(a[0] for a in pushes), 2), "push_refactor_secs": round(sum(a[1] for a in pushes), 2),
            "kernel_refactors": kern, "dual_pushes": len(duals), "dual_refactors": int(sum(a[0] for a in duals)), "dual_secs": round(sum(a[1] for a in duals), 2)}

with ThreadPoolExecutor(2) as ex:
    for row in ex.map(one, [sp for sp in probs if sp[1] not in done]):
        with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
        print(json.dumps(row), flush=True)
