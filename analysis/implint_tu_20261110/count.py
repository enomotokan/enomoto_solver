"""前処理の後の問題で、暗黙の整数の数を判定ごとに数える (ENOMOTO_MIP_DEBUG_IMPLINTCOUNT=stop、src/mip/implint_tu.rs の debug_count)。
結果は count.jsonl (問題ごとに IMPLINTCOUNT の行と前処理の時間)。使い方: python count.py [問題 ...]"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
from probs_all import ALL  # noqa: E402
OUT = D / os.environ.get("COUNT_OUT", "count.jsonl")
probs = sys.argv[1:] or ALL
done = set()
if OUT.exists():
    done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()}
for p in probs:
    if p in done:
        continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_IMPLINTCOUNT="stop")
    if os.environ.get("COUNT_TU2"):
        env["ENOMOTO_MIP_IMPLINT_TU2"] = os.environ["COUNT_TU2"]
    if os.environ.get("IMPLINT_PKG"):
        env["PYTHONPATH"] = os.environ["IMPLINT_PKG"]
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                            "--time-limit", "300", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=900)
        line = [l for l in r.stderr.splitlines() if l.startswith("IMPLINTCOUNT")]
        rec = {"problem": p, "line": line[-1] if line else None, "wall": round(time.time() - t0, 1)}
        if line:
            for k in ["n", "m", "cont", "rowrule", "tu", "tu2nob", "tu2", "tu_not_in_tu2", "costed_cont", "costed_cont_left_tu2"]:
                mm = re.search(rf"\b{k} (\d+)", line[-1])
                rec[k] = int(mm.group(1)) if mm else None
        else:
            rec["tail"] = r.stderr[-400:]
    except Exception as e:  # noqa: BLE001
        rec = {"problem": p, "line": None, "error": str(e)[:300]}
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, rec.get("cont"), rec.get("rowrule"), rec.get("tu"), rec.get("tu2"), rec.get("wall"), flush=True)
