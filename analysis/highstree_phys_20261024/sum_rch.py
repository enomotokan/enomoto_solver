"""batch_rch.sh の要約: python sum_rch.py -> 問題・形・回ごとに状態・目的値・時間・最初の根のカットの終わり (下界・時刻)・
最後の根の下界・ノード数と、HiGHS の最適値 (benchmarks/ の 60 秒の参照) との不一致。"""
import glob, json, re
B = "../../benchmarks/"
ref = {"physiciansched6-2": 49324.0}
for f in [B + "miplib_results_4t_60s_20261020_bc89ad1_highsgap0.json"] + sorted(glob.glob(B + "highsonly16_4t_60s_*.json")):
    for p, v in json.load(open(f)).items():
        h = v.get("highs") or {}
        if str(h.get("status", "")).lower() == "optimal" and h.get("objective") is not None:
            ref.setdefault(p, h["objective"])
print("問題 | 形 | 回 | 状態 | 目的値 | 時間 | 根のカット (最初の根: 下界 @ 時刻、ラウンド) | 最終の根の下界 | ノード | HiGHS")
for p in ["physiciansched6-2", "mzzv42z", "timtab1", "neos-960392", "app1-1", "qiu", "h80x6320d", "nw04", "10teams"]:
    for n in ["def", "rch"]:
        for f in sorted(glob.glob(f"logs/{p}_{n}_[0-9].log")):
            k = f.rsplit("_", 1)[1][:-4]
            rc = "-"; rounds = 0; d = None
            for line in open(f, errors="replace"):
                if rc == "-" and re.match(r"MIP: cut round \d+: \d+ cuts", line):
                    rounds += 1
                m = re.match(r"Root cuts: objective \S+ -> (\S+), .*\(([\d.]+)s\)", line)
                if m and rc == "-":
                    rc = f"{float(m.group(1)):.6g} @ {m.group(2)}s, {rounds}"
                if line.startswith("{"):
                    d = json.loads(line)
            if d is None:
                print(f"{p} | {n} | {k} | (途中)"); continue
            r = ref.get(p)
            mis = " **不一致**" if d["status"] == "optimal" and r is not None and abs(d["objective"] - r) > 1e-6 * max(1, abs(r)) else ""
            ob = d["objective"]
            print(f"{p} | {n} | {k} | {d['status']}{mis} | {ob if ob is None else round(ob, 4)} | {d['time']:.1f} | {rc} | {d['root_bound'] if d['root_bound'] is None else round(d['root_bound'], 4)} | {d['nodes']} | {r}")
