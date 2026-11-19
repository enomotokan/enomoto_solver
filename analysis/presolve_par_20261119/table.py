"""pretime_p58_t60.jsonl を段ごとの表にする。段の分け方は analysis/prepar_20261107/table.py と同じ。
「全体」は analysis/final_20261118 の P (60 秒、4 スレッド、既定 def) の種 1〜3 の秒の中央値 (根の前で止めた前処理の回とは別の回)。
縮約の本体の中の段 (ENOMOTO_PROF_PRESOLVE の PROF_PRESOLVE、かけ直しの分も足す) の合計も出す。
使い方: python table.py [最小秒 (既定 1)]"""
import json, sys, statistics
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "prepar_20261107"))
from table import stages as stages0, KEYS  # noqa: E402

def stages(r):
    """probe は自分を呼び直す (probe の中の probe) ので guard の秒が二重に数えられる。probe は全体から他の段を引いた残りとし、other は 0 にする"""
    s = stages0(r)
    s["probe"] = max(0.0, r.get("total", 0.0) - sum(v for k, v in s.items() if k not in ("probe", "other")))
    s["other"] = 0.0
    return s
mins = float(sys.argv[1]) if len(sys.argv) > 1 else 1.0
rows = [json.loads(l) for l in open(D / "pretime_p58_t60.jsonl") if l.strip()]
tot = defaultdict(list)
for f in sorted((D.parent / "final_20261118").glob("res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def" and isinstance(r["result"], dict) and "time" in r["result"]:
            tot[r["problem"]].append(r["result"]["time"])
full = {p: statistics.median(v) for p, v in tot.items()}
ok = [r for r in rows if "total" in r]
print(f"問題 {len(rows)} (表が出た {len(ok)})")
T = sum(r["total"] for r in ok)
S = {k: sum(stages(r)[k] for r in ok) for k in KEYS}
print(f"前処理の秒の合計 {T:.1f} (中央値 {sorted(r['total'] for r in ok)[len(ok) // 2]:.3f})")
print("段ごとの合計: " + "  ".join(f"{k} {S[k]:.1f} ({100 * S[k] / T:.0f}%)" for k in KEYS))
F = sum(full.get(r["problem"], 0) for r in ok)
print(f"全体 (final_20261118 P def の中央値) の合計 {F:.1f} 秒、前処理の割合 {100 * T / F:.1f}%")
P = defaultdict(float)
for r in ok:
    for k, v in r.get("prof", {}).items():
        if k != "total":
            P[k] += v
PT = sum(r.get("prof", {}).get("total", 0) for r in ok)
print(f"\n縮約の本体 (run_extended_mip、かけ直しを含む) の合計 {PT:.2f} 秒、中の段 (上位):")
for k, v in sorted(P.items(), key=lambda x: -x[1])[:16]:
    print(f"  {k:28s} {v:7.3f} 秒 ({100 * v / max(PT, 1e-9):.0f}%)")
heavy = sorted([r for r in ok if r["total"] >= mins], key=lambda r: -r["total"])
print(f"\n前処理 {mins:g} 秒以上: {len(heavy)} 問")
hdr = "| 問題 | 前処理 | 全体 | 割合 | " + " | ".join(KEYS) + " | 縮約の本体 (PROF) |"
print(hdr)
print("|" + "---|" * (hdr.count("|") - 1))
for r in heavy:
    s = stages(r)
    t = full.get(r["problem"])
    print(f"| {r['problem']} | {r['total']:.1f} | {t:.1f} | {100 * r['total'] / t:.0f}% | " if t else f"| {r['problem']} | {r['total']:.1f} | - | - | ", end="")
    print(" | ".join(f"{s[k]:.1f}" if s[k] >= 0.05 else "" for k in KEYS) + f" | {r.get('prof', {}).get('total', 0):.2f} |")
