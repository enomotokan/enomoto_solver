"""CLQPRB の続き: census_*.jsonl を問題ごと・時点ごとの表にまとめる。使い方: python censum.py ファイル..."""
import json, sys
K = ["int_free", "bin01", "w1_lo_nz", "w2", "w3", "cont_free", "impl_primal", "impl_primal_dual", "impl_primal_fixc", "impl_primal_dual_fixc", "impl_primal_fixc_w1", "cont01_linked", "late01", "late_w1_lo_nz"]
rows = [json.loads(l) for f in sys.argv[1:] for l in open(f) if l.strip()]
tot = {}
print("| 問題 | 時点 | " + " | ".join(K) + " |")
print("|" + "---|" * (len(K) + 2))
for r in rows:
    for c in r.get("census", []):
        at = c["at"][:1]
        for k in K: tot.setdefault((at, k), [0, 0]); tot[(at, k)][0] += c[k]; tot[(at, k)][1] += c[k] > 0
        interesting = any(c[k] for k in K if k not in ("int_free", "bin01", "cont_free"))
        if interesting: print(f"| {r['problem']} | {c['at']} | " + " | ".join(str(c[k]) for k in K) + " |")
print()
print("合計 (本数 / 1 本以上の問題の数):")
for at in "abc":
    print(at, {k: tuple(tot.get((at, k), [0, 0])) for k in K})
print("問題", len(rows), "c の時点がある問題", sum(1 for r in rows if any(c["at"].startswith("c") for c in r.get("census", []))))
