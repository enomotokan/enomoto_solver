"""OBJINT: 前処理の後の問題で目的値の刻みが付くかを、ENOMOTO の判定ごと (整数列だけ・行の規則・HO2・TU2) と HiGHS 1.15.1 の
`Objective function is integral with scale` で比べる。結果は detect.jsonl。使い方: python detect.py [問題 ...]
ENOMOTO は ENOMOTO_MIP_DEBUG_IMPLINTCOUNT=stop (src/mip/implint_tu.rs の debug_count) で前処理の後に止める。HiGHS は 20 秒で止める (ログだけ見る)。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "implint_tu_20261110"))
sys.path.insert(0, str(WT / "scripts"))
from probs_all import ALL  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
EXTRA = ["csched007", "csched008"]
OUT = D / "detect.jsonl"
probs = sys.argv[1:] or ALL + [p for p in EXTRA if p not in ALL]
done = set()
if OUT.exists():
    done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()}
def worker(kind, mps, tl, extra):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(extra)
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 600)
    return r.stdout + r.stderr
for p in probs:
    if p in done:
        continue
    rec = {"problem": p}
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
        out = worker("ours", mps, 300, {"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DEBUG_IMPLINTCOUNT": "stop"})
        ls = [l for l in out.splitlines() if l.startswith("IMPLINTCOUNT")]
        rec["ours"] = ls
        for k in ["step_rowrule", "step_ho2", "step0", "step_tu2"]:
            m = re.search(rf"\b{k} (None|Some\(([-0-9.e]+)\))", " ".join(ls))
            rec[k] = None if not m or m.group(1) == "None" else float(m.group(2))
        for k in ["ho2_extra", "costed_cont_left_ho2", "costed_cont", "cont", "rowrule"]:
            m = re.search(rf"\b{k} (\d+)", " ".join(ls))
            rec[k] = int(m.group(1)) if m else None
        h = worker("highs", mps, 20, {"ENOMOTO_BENCH_HIGHS_LOG": "1"})
        m = re.search(r"Objective function is integral with scale (\S+)", h)
        rec["highs_scale"] = float(m.group(1)) if m else None
        rec["highs_implint"] = [l.strip() for l in h.splitlines() if "mplied integer" in l or "implint" in l.lower()][:3]
    except Exception as e:  # noqa: BLE001
        rec["error"] = str(e)[:300]
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, rec.get("step0"), rec.get("step_rowrule"), rec.get("step_ho2"), rec.get("step_tu2"), "highs", rec.get("highs_scale"), flush=True)
