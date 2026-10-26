"""(c) の要約: python sum_c.py <名前>... -> 問題ごとに状態・目的値・時間・根のサブ MIP の時間の合計 (逐次 + 並列)・回数と、
HiGHS の最適値 (benchmarks/ の 60 秒の参照) との不一致。"""
import glob, json, re, sys, os
B = "../../benchmarks/"
ref = {}
for f in [B + "miplib_results_4t_60s_20261020_bc89ad1_highsgap0.json"] + sorted(glob.glob(B + "highsonly16_4t_60s_*.json")):
    for p, v in json.load(open(f)).items():
        h = v.get("highs") or {}
        if str(h.get("status", "")).lower() == "optimal" and h.get("objective") is not None:
            ref.setdefault(p, h["objective"])
names = sys.argv[1:]
P40 = __import__("subprocess").check_output(["python3", "-c", "import sys; sys.path.insert(0,'../../scripts'); from run_miplib_benchmark import PROBLEMS; print(' '.join(PROBLEMS))"], text=True).split()
P16 = "nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1".split()
tot = {n: [0, 0.0, 0.0, 0] for n in names}
print("問題 | " + " | ".join(f"{n}: 状態 目的値 時間 サブMIP秒(回)" for n in names) + " | HiGHS")
for p in P40 + P16:
    row = []
    for n in names:
        fs = sorted(glob.glob(f"logs/{p}_c_{n}_*.log"))
        if not fs:
            row.append("-"); continue
        st = obj = t = None; sub = 0.0; k = 0
        for line in open(fs[-1], errors="replace"):
            m = re.search(r"sub-MIP(?: \([^)]*\))?: .*?([\d.]+)s$", line)
            if m and ("MIP:   sub-MIP:" in line or "parallel sub-MIP" in line):
                sub += float(m.group(1)); k += 1
            if line.startswith("{"):
                d = json.loads(line); st, obj, t = d["status"], d["objective"], d["time"]
        mis = ""
        r = ref.get(p)
        if st == "optimal" and r is not None and obj is not None and abs(obj - r) > 1e-6 * max(1.0, abs(r)):
            mis = " **不一致**"
        tot[n][0] += st == "optimal"; tot[n][1] += sub; tot[n][3] += k
        tot[n][2] += t or 0
        row.append(f"{st} {obj} {t:.1f} {sub:.1f}({k}){mis}" if t is not None else "(途中)")
    print(f"{p} | " + " | ".join(row) + f" | {ref.get(p)}")
for n in names:
    print(f"計 {n}: 最適 {tot[n][0]}、時間の和 {tot[n][2]:.0f} 秒、サブ MIP の時間の和 {tot[n][1]:.1f} 秒 ({tot[n][3]} 回)")
