"""6 章: mk50 の候補 (解ごとに残した 1 本) と同じ LP 点の zerohalf の候補の平行の数え上げ (run.py の zhcmp、MODKVB_ZHCMP の行の和)。
zhcmp = [zerohalf の候補 (延べ), k2plain の残した数, 平行, 右辺も同じ, CG の式, ほかの残した数, 平行, 右辺も同じ, CG の式]。
k2plain = k = 2・変数上下限の置き換えなし・LP のカットの行なし (zerohalf と同じ種類の組)。使い方: python zhcmp.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
for f in ["root58.jsonl", "rootmitt.jsonl"]:
    R = {}
    for l in open(D / f):
        r = json.loads(l)
        if r["cfg"] in ("mk50z", "mk50cg", "mk50cgb") and "zhcmp" in r:
            R.setdefault((r["cfg"], r.get("rep", 1)), []).append((r["problem"], r["zhcmp"]))
    for (c, rep), L in sorted(R.items()):
        t = [sum(v[i] for _, v in L) for i in range(9)]
        k2, oth = t[1], t[5]
        print(f"{f} {c} 種{rep} ({len(L)} 問): zerohalf 延べ {t[0]}、k2plain 残した {k2} (平行 {t[2]}・右辺も同じ {t[3]}・CG {t[4]})、"
              f"ほか {oth} (平行 {t[6]}・同じ {t[7]}・CG {t[8]})、平行の割合 {100 * (t[2] + t[6]) / max(1, k2 + oth):.1f}%")
        top = sorted(L, key=lambda x: -(x[1][2] + x[1][6]))[:6]
        print("   平行の多い問題:", [(p, v[2] + v[6], v[1] + v[5]) for p, v in top if v[2] + v[6] > 0])
