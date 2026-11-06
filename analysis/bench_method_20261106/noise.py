"""1. 既定どうしの回から、問題ごとの揺れ (1 回の log(t+10) の分散・時間の変動係数・解ける割合) を見積もる。
出力: noise58.json・noiseMitt.json (benchlib.load_noise で読む) と、表を標準出力 (noise.txt に保存)。

既定の組 (同じ既定の回をまとめる。docs/improvement_history.md の既定化の日付で分ける):
  58 問: halve_submip の bench9〜18 は bench ごと、A = bench19・20 (canddet の後・p の前)、B = bench21・22 (p の後・memall の前)、
         C = bench25、D = bench26、E = bench27・28 (POSTSOLVE_CLIP の後・sym/lp の前。間の ZH/POLISH_DEADLINE は期限の後だけ効く)。
         ほかの analysis/*/bench58*.jsonl の def は、同じ記録の置き場 (ディレクトリ) を 1 組とする (同じビルドとみなす。推測)。
  Mittelmann (回 >= 100): B (bench21・22 の rep 115) と E (bench27 の 116・bench28 の 117)。A (bench19 の 113?・bench20 の 114) も足す。
"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
HERE = Path(__file__).resolve().parent
AN = HERE.parent
sys.path.insert(0, str(AN))
import benchlib as bl  # noqa: E402

HS = AN / "halve_submip_20261030"
GROUPS_HS = {f"bench{n}": [n] for n in range(9, 19)}
GROUPS_HS.update({"A(19,20)": [19, 20], "B(21,22)": [21, 22], "C(25)": [25], "D(26)": [26], "E(27,28)": [27, 28]})
RECENT = ["A(19,20)", "B(21,22)", "C(25)", "D(26)", "E(27,28)"]


def hs_rows(n):
    return bl.load(sorted(HS.glob(f"bench{n}_s*.jsonl")))


RAW_OTHER = defaultdict(list)


def other_groups():
    """halve_submip の外の bench58*.jsonl の def を、ディレクトリごとに 1 組。回はファイルと rep で分ける。"""
    out = {}
    for f in sorted(AN.glob("**/bench58*.jsonl")):
        if "halve_submip" in str(f) or "/old/" in str(f):
            continue
        rows = [r for r in bl.load([f]) if r["cfg"] == "def"]
        if not rows:
            continue
        g = out.setdefault(str(f.parent.relative_to(AN)), defaultdict(list))
        per = defaultdict(list)
        for r in rows:
            per[(r["problem"], f.name, r["rep"])].append(r["result"])
        for (p, _, _), rs in per.items():
            g[p].append(bl.lt(rs[-1]))
            RAW_OTHER[p].append(("other:" + str(f.parent.relative_to(AN)), bl.t(rs[-1]), bl.ok(rs[-1])))
    return {k: dict(v) for k, v in out.items()}


def build_groups():
    """(groups58, groupsM, raw58, rawM)。groups は {組: {問題: [log(t+10)]}}。"""
    groups58, groupsM = {}, {}
    raw58 = defaultdict(list)  # 問題 -> [(組, t, ok)]
    rawM = defaultdict(list)
    for gname, ns in GROUPS_HS.items():
        g58, gM = defaultdict(list), defaultdict(list)
        for n in ns:
            for r in hs_rows(n):
                if r["cfg"] != "def":
                    continue
                res = r["result"]
                if r["rep"] >= 100:
                    gM[r["problem"]].append(bl.lt(res))
                    rawM[r["problem"]].append((gname, bl.t(res), bl.ok(res)))
                else:
                    g58[r["problem"]].append(bl.lt(res))
                    raw58[r["problem"]].append((gname, bl.t(res), bl.ok(res)))
        groups58[gname] = dict(g58)
        if gname in RECENT:
            groupsM[gname] = dict(gM)
    oth = other_groups()
    for k, g in oth.items():
        groups58["other:" + k] = g
    for p, v in RAW_OTHER.items():
        raw58[p].extend(v)
    return groups58, groupsM, raw58, rawM


def noise_dict(groups, names, ntop):
    """組 names の def から {"var", "noisy"} を作る (benchlib.report に渡す形)。"""
    nz = bl.noise_from_groups([groups[n] for n in names if n in groups])
    var = {p: v for p, (v, _) in nz.items()}
    return {"var": var, "noisy": sorted(var, key=lambda p: -var[p])[:ntop], "sens": sens_from_groups(groups, names)}


def sens_from_groups(groups, names):
    """感度 s_p = 既定の回の t/(t+10) の平均 (時間切れの回は 0)。"""
    acc = defaultdict(list)
    tl = math.log(bl.TL + bl.SHIFT) - 1e-9
    for n in names:
        for p, xs in groups.get(n, {}).items():
            for x in xs:
                tt = math.exp(x) - bl.SHIFT
                acc[p].append(0.0 if x >= tl else tt / (tt + bl.SHIFT))
    return {p: sum(v) / len(v) for p, v in acc.items()}


def main():
    groups58, groupsM, raw58, rawM = build_groups()

    print("== 組ごとの回の数 (58 問の 1 問あたりの回の数の最大)")
    for k, g in groups58.items():
        if g:
            print(f"  {k:45s} 問題 {len(g):3d}  回 {max(len(v) for v in g.values())}")

    def summarize(groups, raw, names, title, fname, ntop):
        nz = bl.noise_from_groups([groups[n] for n in names if n in groups])
        rows = []
        for p, (v, df) in nz.items():
            rr = [x for x in raw.get(p, []) if x[0] in names]
            ts = [x[1] for x in rr]
            oks = [x[2] for x in rr]
            mt = sum(ts) / len(ts) if ts else float("nan")
            sd = math.sqrt(sum((x - mt) ** 2 for x in ts) / (len(ts) - 1)) if len(ts) > 1 else float("nan")
            rows.append((p, v, df, mt, sd / mt if mt else float("nan"), sum(oks) / len(oks) if oks else float("nan"), len(ts)))
        rows.sort(key=lambda r: -r[1])
        n = len(rows)
        tot = sum(r[1] for r in rows)
        print(f"\n== {title}: 問題 {n}、1 回の log(t+10) の分散の和 {tot:.3f}")
        print(f"  {'問題':28s} {'σ(log)':>7s} {'自由度':>5s} {'平均t':>6s} {'CV':>5s} {'解ける':>6s} {'回':>3s} {'分散の割合':>9s} 累積")
        cum = 0
        for p, v, df, mt, cv, sr, nr in rows:
            cum += v
            if v > 1e-4 or nr:
                print(f"  {p:28s} {math.sqrt(v):7.3f} {df:5d} {mt:6.1f} {cv:5.2f} {sr:6.0%} {nr:3d} {v / tot:9.1%} {cum / tot:5.1%}")
        noisy = [r[0] for r in rows[:ntop]]
        json.dump({"var": {r[0]: r[1] for r in rows}, "df": {r[0]: r[2] for r in rows},
                   "sens": sens_from_groups(groups, names),
                   "mean_t": {r[0]: r[3] for r in rows}, "solve_rate": {r[0]: r[5] for r in rows},
                   "noisy": noisy, "groups": [n for n in names if n in groups]},
                  open(HERE / fname, "w"), ensure_ascii=False, indent=1)
        return rows

    rows58 = summarize(groups58, raw58, RECENT + [k for k in groups58 if k.startswith("other:")],
                       "58 問 (A〜E と他の記録の def)", "noise58.json", 12)
    summarize(groups58, raw58, [k for k in groups58 if k.startswith("bench")], "58 問 (古い bench9〜18 だけ、確かめ用)", "noise58_old.json", 12)
    summarize(groupsM, rawM, RECENT, "Mittelmann (A・B・E の def どうし)", "noiseMitt.json", 20)

    # 組の中 (同じ bench の 2 回) と組をまたぐ (bench が違う) 分散の比べ: B・E
    print("\n== 同じ bench の 2 回と、bench をまたぐ (同じ既定) 回の差の分散 (58 問の和)")
    for gname, ns in (("A", [19, 20]), ("B", [21, 22]), ("E", [27, 28])):
        per = {}
        for n in ns:
            for r in hs_rows(n):
                if r["cfg"] == "def" and r["rep"] < 100:
                    per.setdefault(n, defaultdict(dict))[r["problem"]][r["rep"]] = bl.lt(r["result"])
        n1, n2 = ns
        within, across = [], []
        for p in per[n1]:
            for n in ns:
                v = list(per[n][p].values())
                if len(v) == 2:
                    within.append((v[0] - v[1]) ** 2 / 2)
            a, b = list(per[n1][p].values()), list(per[n2].get(p, {}).values())
            for x in a:
                for y in b:
                    across.append((x - y) ** 2 / 2 / (len(a) * len(b)))
        npb = len(per[n1])
        print(f"  {gname}: 同じ bench の中 {sum(within) / 2:.3f} (1 回の分散の和)、bench をまたぐ {sum(across):.3f}  (問題 {npb})")


if __name__ == "__main__":
    main()
