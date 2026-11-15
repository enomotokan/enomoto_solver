"""pretime_*.jsonl を段ごとの表にする (analysis/prepar_20261107.md)。
使い方: python table.py pretime_X.jsonl [res の glob (600 秒の全体の秒、既定の 'ours')] [最小秒 (既定 5)]
段: reduce (縮約の本体) / coef (係数の強化・行の正規化・平行な行) / redo (かけ直しの縮約・係数: 入れ子の前処理の build+reduce+coef) /
qprobe (早めのプロービング) / fj (早めの FJ、4 スレッド) / lpfirst (LP による固定の最初の LP) / lpfix (LP による固定の残り) /
probe (プロービング) / clq (クリーク行の拡張) / sym (対称性の検出) / hnf (HNFPRE) / other (残り: 行列の組み立て・写しなど)"""
import glob, json, sys
from pathlib import Path
def g(r, k):
    return r.get(k, 0.0) + r.get(k + "@re", 0.0)

def stages(r):
    s = {
        "reduce": r.get("reduce", 0.0), "coef": r.get("coef_merge", 0.0),
        "redo": r.get("build@re", 0.0) + r.get("reduce@re", 0.0) + r.get("coef_merge@re", 0.0),
        "qprobe": g(r, "qprobe"), "fj": g(r, "fj_early"), "lpfirst": g(r, "lpprobe_first"),
        "lpfix": g(r, "lpprobe") - g(r, "lpprobe_first"), "probe": g(r, "probe"), "clq": g(r, "clqext"), "sym": g(r, "sym"), "hnf": g(r, "hnfpre"),
    }
    s["other"] = max(0.0, r.get("total", 0.0) - sum(s.values()))
    return s

KEYS = ["reduce", "coef", "redo", "qprobe", "fj", "lpfirst", "lpfix", "probe", "clq", "sym", "hnf", "other"]

if __name__ == "__main__":
    rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
    tot600 = {}
    if len(sys.argv) > 2 and sys.argv[2] != "-":
        for f in sorted(glob.glob(sys.argv[2])):
            for l in open(f):
                r = json.loads(l)
                if r["solver"] == "ours" and isinstance(r["result"], dict) and "time" in r["result"]:
                    tot600.setdefault(r["problem"], []).append(r["result"]["time"])
    mins = float(sys.argv[3]) if len(sys.argv) > 3 else 5.0

    ok = [r for r in rows if "total" in r]
    bad = [r for r in rows if "total" not in r]
    print(f"問題 {len(rows)} (表が出た {len(ok)}、出なかった {len(bad)}: {', '.join(r['problem'] + '(' + str(r.get('rc')) + ')' for r in bad)})")
    S = {k: sum(stages(r)[k] for r in ok) for k in KEYS}
    T = sum(r["total"] for r in ok)
    print(f"前処理の秒の合計 {T:.1f} (1 問の平均 {T / max(1, len(ok)):.2f}、中央値 {sorted(r['total'] for r in ok)[len(ok) // 2]:.3f})")
    print("段ごとの合計: " + "  ".join(f"{k} {S[k]:.1f} ({100 * S[k] / T:.0f}%)" for k in KEYS))
    # スレッドの使われ方: fj は 4 本、最初の LP は 1〜4 本 (同時実行)、ほかは 1 本
    busy_lo = sum(s["fj"] * 4 + (r["total"] - s["fj"]) for r in ok for s in [stages(r)])
    busy_hi = sum(s["fj"] * 4 + s["lpfirst"] * 4 + (r["total"] - s["fj"] - s["lpfirst"]) for r in ok for s in [stages(r)])
    print(f"4 スレッドのうち働いている割合 (推定): {100 * busy_lo / (4 * T):.0f}〜{100 * busy_hi / (4 * T):.0f}%")
    heavy = sorted([r for r in ok if r["total"] >= mins], key=lambda r: -r["total"])
    print(f"\n前処理 {mins:g} 秒以上: {len(heavy)} 問")
    hdr = "| 問題 | 前処理 | 全体 | 割合 | " + " | ".join(KEYS) + " | 重い段 | FJ 終了 |"
    print(hdr)
    print("|" + "---|" * (hdr.count("|") - 1))
    for r in heavy:
        s = stages(r)
        t6 = tot600.get(r["problem"])
        t6s = f"{sum(t6) / len(t6):.0f}" if t6 else "-"
        frac = f"{100 * r['total'] / (sum(t6) / len(t6)):.0f}%" if t6 else "-"
        top = sorted(KEYS, key=lambda k: -s[k])[:2]
        print(f"| {r['problem']} | {r['total']:.1f} | {t6s} | {frac} | " + " | ".join(f"{s[k]:.1f}" if s[k] >= 0.05 else "" for k in KEYS) + f" | {', '.join(top)} | {r.get('@fj_early_end', '') and round(r['@fj_early_end'], 1)} |")
