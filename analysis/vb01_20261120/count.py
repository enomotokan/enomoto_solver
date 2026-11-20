"""VB01 が働く問題の数え上げ: python count.py SET [並列数] (SET は P (MIPLIB の 58 問) か M (Mittelmann 186 問) か、問題名をカンマで並べたもの)。
各問題を ENOMOTO_MIP_VB01=1・ENOMOTO_MIP_LOG=1・1 スレッド・種 1・時間の上限 TL 秒 (環境変数 TL、既定 60) で解き、ログの VB01 の行
(使ったか・型ごとの固定率・サブ MIP の結果・ヒューリスティクスの時間) を count_SET.jsonl に 1 問 1 行で足す。途中から再開できる。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
SET = sys.argv[1]
NPAR = int(sys.argv[2]) if len(sys.argv) > 2 else 2
TL = int(os.environ.get("TL", "60"))
MODE = os.environ.get("MODE", "1")  # ENOMOTO_MIP_VB01 の値 (1 = r: 根のヒューリスティクスの後、b: 根の LP の前)
if SET == "P":
    from probs import PROBS as probs  # noqa: E402
elif SET == "M":
    # 構造の数え上げ (count_struct.py) で候補になった問題だけ
    probs = [r["problem"] for l in open(D / "count_struct_M.jsonl") if l.strip() for r in [json.loads(l)] if r.get("cand")]
else:
    probs = SET.split(",")
OUT = D / f"count_{SET if SET in ('P', 'M') else 'x'}{'' if MODE == '1' else '_' + MODE}.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
(D / "logs").mkdir(exist_ok=True)

def one(p):
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        return {"problem": p, "status": "fetch_error", "error": str(e)[:200]}
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_VB01": MODE, "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
    log = D / "logs" / f"count_{p}_{MODE}.log"
    t0 = time.time()
    with open(log, "w") as err:
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "1"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=TL + 300)
            out = pr.stdout
        except subprocess.TimeoutExpired:
            out = ""
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash"}
    lines = [l.strip() for l in open(log, errors="replace") if "VB01" in l]
    variants = {}
    for l in lines:
        m = re.search(r"(VB01 \([a-z, ]+\)): (\d+) fixings, (\d+) backtracks(, infeasible)?, fixing rate ([0-9.]+)(?: \(all columns ([0-9.]+)\))?", l)
        if m:
            variants[m.group(1)] = {"fix": int(m.group(2)), "back": int(m.group(3)), "dead": bool(m.group(4)), "rate": float(m.group(5)), "rall": float(m.group(6)) if m.group(6) else None}
    heur = [l for l in lines if "root heuristic VB01" in l or "VB01 before the root LP" in l]
    notused = [l for l in lines if "not used" in l]
    nsub = sum(1 for v in variants.values() if not v["dead"] and v["rate"] >= 0.65 and (v["rall"] is None or v["rall"] >= 0.65))
    return {"problem": p, "result": res, "wall": round(time.time() - t0, 1), "variants": variants, "nsub": nsub,
            "heur": heur, "notused": notused[:1], "ran": bool(lines)}

todo = [p for p in probs if p not in done]
with ThreadPoolExecutor(NPAR) as ex:
    for r in ex.map(one, todo):
        with open(OUT, "a") as f:
            f.write(json.dumps(r) + "\n")
        print(r["problem"], r.get("result", {}).get("status"), r.get("nsub"), r.get("heur"), r.get("notused"), flush=True)
