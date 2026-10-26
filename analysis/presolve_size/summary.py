"""sizes.jsonl の勝ち負けの集計 (行・列・非零は 1% 以上の差、LP は相対 1e-6 以上の差を数える)。使い方: python3 summary.py sizes.jsonl"""
import sys, json
R = [json.loads(l) for l in open(sys.argv[1])]
for k in ("rows", "cols", "nnz"):
    hs = [r["name"] for r in R if r["h"] and r["e2"] and r["e2"][k] > 1.01 * r["h"][k]]
    es = [r["name"] for r in R if r["h"] and r["e2"] and r["h"][k] > 1.01 * r["e2"][k]]
    print(f"{k}: HiGHS が小さい {len(hs)} 問 {hs}\n      ENOMOTO が小さい {len(es)} 問 {es}")
hl = [r["name"] for r in R if r["h"] and r["e2"] and r["h"]["lp"] is not None and r["e2"]["lp"] is not None and r["h"]["lp"] - r["e2"]["lp"] > 1e-6 * max(1, abs(r["h"]["lp"]))]
el = [r["name"] for r in R if r["h"] and r["e2"] and r["h"]["lp"] is not None and r["e2"]["lp"] is not None and r["e2"]["lp"] - r["h"]["lp"] > 1e-6 * max(1, abs(r["h"]["lp"]))]
print(f"LP: HiGHS が強い {len(hl)} 問 {hl}\n    ENOMOTO が強い {len(el)} 問 {el}")
