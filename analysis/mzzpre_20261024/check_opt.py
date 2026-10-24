"""ログの最後の JSON の「最適」を、benchmarks/*.json の HiGHS の最適値と比べる: python check_opt.py logs/*.log
HiGHS が最適と言った回の目的値 (問題ごと) を参照値にする。違えば MISMATCH を出す。"""
import glob, json, sys, os, re
ref = {}
for f in glob.glob(os.path.join(os.path.dirname(__file__), "../../benchmarks/*.json")):
    try: d = json.load(open(f))
    except Exception: continue
    if not isinstance(d, dict): continue
    for p, r in d.items():
        h = r.get("highs") if isinstance(r, dict) else None
        if isinstance(h, dict) and str(h.get("status", "")).lower().startswith("optimal") and h.get("objective") is not None:
            ref.setdefault(p, h["objective"])
for f in sys.argv[1:]:
    p = re.sub(r"_(s_def|s_rf|[a-z0-9]+)_\d+\.log$", "", os.path.basename(f))
    last = [l for l in open(f, errors="replace") if l.startswith("{")]
    if not last: continue
    r = json.loads(last[-1])
    if r.get("status") != "optimal": continue
    o = r["objective"]; v = ref.get(p)
    tag = "noref" if v is None else ("ok" if abs(o - v) <= 1e-6 * max(1, abs(v)) else "MISMATCH")
    print(f"{os.path.basename(f)}\t{o}\t{v}\t{tag}")
