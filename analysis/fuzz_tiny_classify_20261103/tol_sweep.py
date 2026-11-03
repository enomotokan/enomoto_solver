"""食い違いの seed ごとに、行の許容誤差 (0, 1e-9, 2e-9, 3e-9, 1e-6) を変えた全列挙の最適値を出す (classify.py の best_by_enum)。"""
import sys, json
sys.argv = ["classify.py", "0"]
exec(open("analysis/fuzz_tiny_classify_20261103/classify.py").read().split("N = int(sys.argv[2])")[0])
TOLS = [0.0, 1e-9, 2e-9, 3e-9, 1e-6]
print("seed highs ours " + " ".join("tol%.0e" % t for t in TOLS))
for l in open("analysis/fuzz_tiny_classify_20261103/classify_400.jsonl"):
    d = json.loads(l); lp = gen(d["seed"])
    print(d["seed"], "%.6g" % d["highs_obj"], "%.6g" % d["ours_obj"], " ".join("%.6g" % best_by_enum(lp, t)[0] for t in TOLS), flush=True)
