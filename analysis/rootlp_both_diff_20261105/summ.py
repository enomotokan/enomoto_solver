"""ROOTLP_BOTH の結果 (run.py の jsonl) を表にする。使い方: python summ.py ラベル=ファイル.jsonl ... > summ.md"""
import json, sys, statistics as S

def q(v, ps=(0.1, 0.25, 0.5, 0.75, 0.9)):
    v = sorted(v)
    if not v:
        return "-"
    return " / ".join(f"{v[min(len(v) - 1, int(p * len(v)))]:.3g}" for p in ps)

def load(fn):
    return [json.loads(l) for l in open(fn)]

allrecs = []
for arg in sys.argv[1:]:
    lab, fn = arg.split("=")
    recs = load(fn)
    for r in recs:
        r["set"] = lab
    allrecs += recs
    print(f"\n### {lab} ({len(recs)} 問)\n")
    print("| 問題 | 列 | 行 | 主の根は基底なし | 勝ち | 二段 (秒) | 内点法 (秒) | クロスオーバー (秒) | 内点法+XO 計 (秒) | 負け/勝ち | 待つと増える秒 | 目的値の相対差 | 値の違う列 (割合) | うち整数列 | 分数の列 二段 / 内点法 / 共通 (Jaccard) | 基底の違う列 / 行 |")
    print("|---|---:|---:|:-:|:-:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|")
    for r in recs:
        if "error" in r or r.get("winner") == "none":
            print(f"| {r['problem']} | {r.get('n','')} | {r.get('m','')} | | {r.get('winner','-')} | {r.get('t_simplex','')} | | | {r.get('t_i','')} | | | | | | {r.get('error', r.get('st_s','') + ' ' + r.get('st_i',''))[:120]} | |")
            continue
        ts, ti = r["t_simplex"], r["t_i"]
        tipm = r["t_ipm"]
        xo = (r["t_xo"] - tipm) if tipm is not None else None
        both = r["st_s"] == "Optimal" and r["st_i"] == "Some(Optimal)"
        if both:
            w, l = (ts, ti) if r["winner"] == "simplex" else (r["t_xo"], ts)
            ratio, extra = l / max(w, 1e-4), l - w
            r["ratio"], r["extra"], r["t_win"] = ratio, extra, w
            od = abs(r["obj_s"] - r["obj_i"]) / (1 + abs(r["obj_s"]))
            fu = len(set()) if False else None
            un = r["frac_s"] + r["frac_i"] - r["frac_both"]
            jac = r["frac_both"] / un if un else 1.0
            r["jac"] = jac
            r["dfrac"] = r["ndiff"] / max(r["n"], 1)
            print(f"| {r['problem']} | {r['n']} | {r['m']} | {'y' if r['main_cold'] else 'n'} | {'二段' if r['winner']=='simplex' else '内点法'} | {ts:.3f} | {tipm if tipm is not None else '-'} | {xo if xo is None else round(xo,3)} | {ti:.3f} | {ratio:.2f} | {extra:.3f} | {od:.1e} | {r['ndiff']} ({100*r['dfrac']:.1f}%) | {r['ndiff_int']} | {r['frac_s']} / {r['frac_i']} / {r['frac_both']} ({jac:.2f}) | {r['basis_diff_cols']} / {r['basis_diff_rows']} |")
        else:
            w = ts if r["winner"] == "simplex" else r["t_xo"]
            l = ti if r["winner"] == "simplex" else ts
            r["ratio_f"], r["extra_f"] = l / max(w, 1e-4), l - w
            print(f"| {r['problem']} | {r['n']} | {r['m']} | {'y' if r['main_cold'] else 'n'} | {r['winner']} | {ts:.3f} | {tipm} | {xo} | {ti:.3f} | | | | | | 片方だけ最適: {r['st_s']} / {r['st_i']} | |")

def dist(recs, title):
    ok = [r for r in recs if "ratio" in r]
    print(f"\n### 分布: {title} (両方が最適 {len(ok)} 問 / {len(recs)} 問)\n")
    if not ok:
        return
    sw = sum(r["winner"] == "simplex" for r in ok)
    print(f"- 勝ち: 二段解法 {sw} 問、内点法 + クロスオーバー {len(ok) - sw} 問")
    print(f"- 同じ点 (値の違う列 0) {sum(r['ndiff'] == 0 for r in ok)} 問 ({100*sum(r['ndiff'] == 0 for r in ok)/len(ok):.0f}%)、うち基底まで同じ {sum(r['ndiff'] == 0 and r['basis_diff_cols'] + r['basis_diff_rows'] == 0 for r in ok)} 問")
    print(f"- 値の違う列の割合 (10/25/50/75/90%): {q([100*r['dfrac'] for r in ok])} %  (中央値 {100*S.median(r['dfrac'] for r in ok):.2f}%)")
    print(f"- 分数の整数列の集合の Jaccard 係数 (10/25/50/75/90%): {q([r['jac'] for r in ok])}")
    print(f"- 分数の整数列の数の比 内点法 / 二段 (10/25/50/75/90%): {q([r['frac_i']/max(r['frac_s'],1) for r in ok])}")
    print(f"- 基底の違う変数 (列 + 行) の割合 / 行の数 (10/25/50/75/90%): {q([100*(r['basis_diff_cols']+r['basis_diff_rows'])/2/max(r['m'],1) for r in ok])} %")
    print(f"- 負けた方 / 勝った方 の時間の比 (10/25/50/75/90%): {q([r['ratio'] for r in ok])}")
    print(f"- 負けた方を待つと増える秒 (10/25/50/75/90%): {q([r['extra'] for r in ok])}")
    for k in (1.5, 2, 3, 5, 10):
        c = [r for r in ok if r["ratio"] <= k]
        print(f"  - 比 ≤ {k}: {len(c)} 問 ({100*len(c)/len(ok):.0f}%)、待つ秒の合計 {sum(r['extra'] for r in c):.2f} (勝った方の時間の合計 {sum(r['t_win'] for r in c):.2f})")
    fl = [r for r in recs if "ratio_f" in r]
    if fl:
        print(f"- 片方だけ最適 {len(fl)} 問: " + "、".join(f"{r['problem']} (負けた方は {r['st_i'] if r['winner']=='simplex' else r['st_s']}、待つと +{r['extra_f']:.1f} 秒 = {r['ratio_f']:.1f} 倍)" for r in fl))
    for a in (0.1, 0.5, 1, 5):
        c = [r for r in ok if r["extra"] <= a]
        print(f"  - 増える秒 ≤ {a}: {len(c)} 問 ({100*len(c)/len(ok):.0f}%)")

for lab in dict.fromkeys(r["set"] for r in allrecs):
    dist([r for r in allrecs if r["set"] == lab], lab)
dist(allrecs, "全部")
big = [r for r in allrecs if "t_win" in r and r["t_win"] >= 1.0]
dist(big, "勝った方が 1 秒以上")
