"""2〜4: SGM の回ごとの揺れ・検出力・回数の案と、既存の比べ (bench20〜28、11-06 の 58 問 2 回の記録) の計算し直し。
出力は標準出力 (analyze.txt に保存)。"""
import math, sys
from collections import defaultdict
from pathlib import Path
HERE = Path(__file__).resolve().parent
AN = HERE.parent
sys.path.insert(0, str(AN)); sys.path.insert(0, str(HERE))
import benchlib as bl  # noqa: E402
import noise as nz  # noqa: E402

groups58, groupsM, raw58, rawM = nz.build_groups()
NAMES58 = nz.RECENT + [k for k in groups58 if k.startswith("other:")]
N58 = nz.noise_dict(groups58, NAMES58, 12)
NM = nz.noise_dict(groupsM, nz.RECENT, 20)
Z = 1.959964 + 0.841621  # 両側 5%・検出力 80%


def mean_t(raw, names):
    m = {}
    for p, v in raw.items():
        ts = [x[1] for x in v if x[0] in names]
        if ts:
            m[p] = sum(ts) / len(ts)
    return m


def part1():
    print("=" * 100)
    print("1. SGM の回ごとの揺れ")
    for title, N, raw, names, sg in (("58 問", N58, raw58, NAMES58, 16.5), ("Mittelmann 186 問", NM, rawM, nz.RECENT, 42.5)):
        var = N["var"]; n = len(var); V = sum(var.values())
        s1 = math.sqrt(V) / n
        print(f"\n[{title}] 問題 {n}、1 回の log(t+10) の分散の和 V = {V:.3f}、SGM の 1 回の揺れ σ_run = √V/n = {s1:.4f} (log)")
        print(f"  SGM≈{sg} 秒として、SGM の % に直すと 1 回 ±{bl.sgm_pct(sg, s1):.2%}")
        print("  k 回の平均の標準誤差 (SGM %):  " + "  ".join(f"k={k}: ±{bl.sgm_pct(sg, s1 / math.sqrt(k)):.2%}" for k in (1, 2, 3, 4, 5, 8)))
        print("  2 設定を k 回ずつ比べたときの差の標準誤差 (SGM %): " + "  ".join(f"k={k}: ±{bl.sgm_pct(sg, s1 * math.sqrt(2 / k)):.2%}" for k in (1, 2, 3, 4, 5, 8)))
        top = N["noisy"]
        vt = sum(var[p] for p in top)
        print(f"  揺れの大きい {len(top)} 問で分散の {vt / V:.0%}: {', '.join(top)}")
        mt = mean_t(raw, names)
        tl = [p for p in var if mt.get(p, 0) >= 59.9]
        print(f"  既定でいつも時間切れの問題 {len(tl)} (揺れ 0、回すと 1 回あたり {60 * len(tl):.0f} 秒): {', '.join(sorted(tl)) if len(tl) < 25 else '(略)'}")
    # 実測: 同じ bench の def の 2 回の SGM の差
    print("\n[実測] 同じ bench の既定の 2 回 (58 問) の SGM の差:")
    diffs = []
    for n in list(range(9, 29)):
        rows = [r for r in nz.hs_rows(n) if r["cfg"] == "def" and r["rep"] < 100]
        if not rows:
            continue
        per = defaultdict(dict)
        for r in rows:
            per[r["rep"]][r["problem"]] = r["result"]
        reps = sorted(per)
        if len(reps) != 2:
            continue
        ps = sorted(set(per[reps[0]]) & set(per[reps[1]]))
        a = bl.sgm([bl.t(per[reps[0]][p]) for p in ps]); b = bl.sgm([bl.t(per[reps[1]][p]) for p in ps])
        na = sum(bl.ok(per[reps[0]][p]) for p in ps); nb = sum(bl.ok(per[reps[1]][p]) for p in ps)
        diffs.append(math.log((b + 10) / (a + 10)))
        print(f"  bench{n}: {a:.2f} ({na}) / {b:.2f} ({nb})  差 {b / a - 1:+.1%}")
    sd = math.sqrt(sum(d * d for d in diffs) / len(diffs))
    print(f"  {len(diffs)} 組の差の二乗平均の平方根 {sd:.4f} (log) → 1 回の σ_run の実測 {sd / math.sqrt(2):.4f}"
          f" (見積もり {math.sqrt(sum(N58['var'].values())) / len(N58['var']):.4f})")


def design(var, sens, kmap, w=None):
    """各問題 p を k_p 回ずつ 2 設定で回したとき、重みつきの平均の差の (標準誤差, 感度の重みつき平均)。
    共通の割合 r のモデル (Δ_p ≈ r s_p) で、信号は r × 感度の重みつき平均。"""
    ps = [p for p in var if kmap.get(p, 0) > 0 and (w is None or w[p] > 0)]
    ws = {p: (1.0 if w is None else w[p]) for p in ps}
    sw = sum(ws.values())
    v = sum(ws[p] ** 2 * 2 * var[p] / kmap[p] for p in ps) / sw ** 2
    s = sum(ws[p] * sens.get(p, 0) for p in ps) / sw
    return math.sqrt(v), s


def part2():
    print("\n" + "=" * 100)
    print("3. 検出力: 全問の SGM で ±2%・±3% に当たる差を両側 5%・検出力 80% で見分けられるか (共通の割合 r のモデル: 解ける回の時間が")
    print("   すべて同じ割合 r で変わる、Δ_p ≈ r s_p、s_p = t/(t+10) で時間切れは 0。重みの案もこの r を推めるものとして比べる)")
    for title, N, raw, names, sg, ntop in (("58 問", N58, raw58, NAMES58, 16.5, 12), ("Mittelmann", NM, rawM, nz.RECENT, 42.5, 20)):
        var = N["var"]; sens = N["sens"]; mt = mean_t(raw, names)
        top = N["noisy"][:ntop]
        alw = [p for p in var if mt.get(p, 0) >= 59.9]
        sbar = sum(sens.get(p, 0) for p in var) / len(var)
        print(f"\n[{title}] (SGM≈{sg} 秒、感度の平均 {sbar:.3f}、時間は 1 設定あたりの計算の秒数の見積もり = Σ k_p × 平均時間)")
        print(f"  {'案':48s} {'雑音/信号':>9s} {'2%の検出力':>9s} {'3%の検出力':>9s} {'時間(秒)':>8s} {'2%に要る倍':>9s}")
        d2, d3 = bl.log_from_pct(sg, 0.02), bl.log_from_pct(sg, 0.03)
        r2, r3 = d2 / sbar, d3 / sbar
        def show(label, kmap, w=None, cost_k=None):
            se, s = design(var, sens, kmap, w)
            pw = lambda r: 0.5 * (1 + math.erf((r * s / se - 1.959964) / math.sqrt(2)))
            ck = cost_k or kmap
            cost = sum(ck.get(p, 0) * mt.get(p, 30) for p in var)
            # SGM % に直した標準誤差 (信号の尺度を全問の SGM にそろえる)
            se_pct = bl.sgm_pct(sg, se / s * sbar)
            need = (Z * se / (r2 * s)) ** 2
            print(f"  {label:48s} ±{se_pct:7.2%} {pw(r2):9.0%} {pw(r3):9.0%} {cost:8.0f} {need:9.1f}")
        for k in (1, 2, 3, 4, 6, 8):
            show(f"全問 {k} 回 (今の SGM)", {p: k for p in var})
        show("全問 2 回・いつも時間切れの問題は 1 回 (SGM は同じ)", {p: 2 for p in var}, cost_k={p: (1 if p in alw else 2) for p in var})
        for kk in (4, 5, 8):
            show(f"揺れの大きい {ntop} 問 {kk} 回・ほか 2 回", {p: (kk if p in top else 2) for p in var},
                 cost_k={p: (kk if p in top else (1 if p in alw else 2)) for p in var})
        budget = sum((1 if p in alw else 2) * mt.get(p, 30) for p in var)
        sig = {p: math.sqrt(var[p]) * 1 for p in var}
        # Neyman: 等しい重みの SGM の分散を予算の中で最小にする k_p ∝ σ_p/√c_p (2〜8 回に丸める)
        base = sum(sig[p] * math.sqrt(mt.get(p, 30)) for p in var if p not in alw)
        kn = {p: (1 if p in alw else min(8, max(2, round(budget * sig[p] / math.sqrt(mt.get(p, 30)) / base)))) for p in var}
        show("Neyman 配分 (k∝σ/√時間、2〜8 回、時間切れは 1)", kn)
        print("    Neyman の回の数 (3 回以上): " + ", ".join(f"{p} {k}" for p, k in sorted(kn.items(), key=lambda x: -x[1]) if k > 2))
        show(f"(a1) 揺れの大きい {ntop} 問を除いた SGM・2 回", {p: 2 for p in var}, w={p: (0.0 if p in top else 1.0) for p in var},
             cost_k={p: (0 if p in top else (1 if p in alw else 2)) for p in var})
        show("(a2) 感度/揺れの重み・2 回", {p: 2 for p in var}, w={p: sens.get(p, 0) / (var[p] + 0.002) for p in var},
             cost_k={p: (1 if p in alw else 2) for p in var})
        show("(a2) 感度/揺れの重み・揺れの大きい問題 5 回", {p: (5 if p in top else 2) for p in var},
             w={p: sens.get(p, 0) / (var[p] / (2.5) + 0.002) if p in top else sens.get(p, 0) / (var[p] + 0.002) for p in var},
             cost_k={p: (5 if p in top else (1 if p in alw else 2)) for p in var})


# ------------------------------------------------------------ 4. 既存の比べの計算し直し
HSB = {20: ("A(19,20)", ["p", "po", "pog"]), 21: ("B(21,22)", ["l1", "l2", "l3", "l123"]),
       22: ("B(21,22)", ["siblu1r", "mem2p", "margin", "memall"]), 25: ("C(25)", ["mem3", "forceq", "fstop", "all4"]),
       26: ("D(26)", ["rob5", "rootipm", "stallrw", "lnsx"]), 27: ("E(27,28)", ["ipmx", "combb", "ipmcombb"]),
       28: ("E(27,28)", ["sym", "lp", "prr", "all28"])}


def loo(groups, names, drop, ntop):
    return nz.noise_dict(groups, [n for n in names if n != drop], ntop)


def summary_line(r, kind):
    sa = r["sgmA"]
    m, lo, hi, p = r[kind][:4]
    if kind == "drop":
        sa = r["drop"][4]
    return f"{bl.sgm_pct(sa, m):+.1%} [{bl.sgm_pct(sa, lo):+.1%},{bl.sgm_pct(sa, hi):+.1%}] p={p:.2f}"


LUCK = []
POOLED = []


def pooled_ref():
    """同じ既定の組 (A・B・E) の def をすべて参照にした、58 問の対にしない比べ。"""
    print("\n" + "=" * 100)
    print("4b. 同じ既定の組の def をすべて (4 回) 参照にした比べ (58 問)。def の 1 回の運を薄める")
    for g, benches in (("A(19,20)", {20: ["p", "po", "pog"]}), ("B(21,22)", {21: ["l1", "l2", "l3", "l123"], 22: ["siblu1r", "mem2p", "margin", "memall"]}),
                       ("E(27,28)", {27: ["ipmx", "combb", "ipmcombb"], 28: ["sym", "lp", "prr", "all28"]})):
        ref = groups58[g]
        sa = bl.sgm([math.exp(x) - 10 for v in ref.values() for x in v])
        for n, cfgs in benches.items():
            R = bl.table(nz.hs_rows(n))
            own = defaultdict(list)
            for k, d in R.items():
                if k[1] < 100 and "def" in d:
                    own[k[0]].append(bl.lt(d["def"]))
            for c in cfgs:
                tr = defaultdict(list)
                for k, d in R.items():
                    if k[1] < 100 and c in d:
                        tr[k[0]].append(bl.lt(d[c]))
                pp = bl.pooled_cmp(tr, ref)
                m, (lo, hi) = bl.ratio_ci(pp)
                pp0 = bl.pooled_cmp(tr, own)
                m0 = sum(v for v, _ in pp0.values()) / len(pp0)
                line = (f"  {g} bench{n} {c:9s} 自分の def 2 回と: {bl.sgm_pct(sa, m0):+.1%}   組の def 4 回と: {bl.sgm_pct(sa, m):+.1%} "
                        f"[{bl.sgm_pct(sa, lo):+.1%}, {bl.sgm_pct(sa, hi):+.1%}]")
                print(line)
                POOLED.append((f"bench{n}", c, bl.sgm_pct(sa, m0), bl.sgm_pct(sa, m), bl.sgm_pct(sa, lo), bl.sgm_pct(sa, hi)))


def part3():
    print("\n" + "=" * 100)
    print("4. 既存の比べの計算し直し (揺れの表はその bench の組を除いた def から作る)")
    quiet = lambda *a, **k: None
    table = []
    for n, (g, cfgs) in HSB.items():
        R = bl.table(nz.hs_rows(n))
        k58 = [k for k in R if k[1] < 100]
        kM = [k for k in R if k[1] >= 100]
        n58 = loo(groups58, NAMES58, g, 12)
        nM = loo(groupsM, nz.RECENT, g, 20)
        for lab, kk in (("58 問", k58), ("Mittelmann", kM)):
            m, top = bl.def_luck(R, "def", cfgs, kk)
            sa = bl.sgm([bl.t(R[k]["def"]) for k in kk if "def" in R[k]])
            print(f"\n*** bench{n} {lab}: 既定の回 − 試作の中央値 = {m:+.4f} (log、SGM 相当 {bl.sgm_pct(sa, m):+.1%})  大きい順: "
                  + ", ".join(f"{k[0]} r{k[1]} {x:+.2f}" for x, k in top[:4]))
            LUCK.append((f"bench{n}", lab, m, bl.sgm_pct(sa, m), top[:3]))
        for c in cfgs:
            print(f"\n--- bench{n} {c}")
            r58 = bl.report(R, "def", c, k58, noise=n58, label=f"bench{n} 58問 {c}")
            rM = bl.report(R, "def", c, kM, noise=nM, label=f"bench{n} Mitt {c}", show_root=True)
            table.append((f"bench{n}", c, r58, rM))
    # 11-06 などの 58 問 2 回の記録
    others = [("highs_slow_20261106/S1", ["s1all"]), ("highs_slow_20261106/S2", ["stopwinc", "inc"]),
              ("highs_slow_20261106/S4", ["rfeqff", "s4c"]), ("highs_slow_20261106/S5", ["trforce"]),
              ("highs_slow_20261106/S6", ["jcx", "tcap", "c0jc", "p2r"]), ("prr_harm_20261106", ["prr"]),
              ("rownorm_20261106", ["rownorm"]), ("cutloop_stop_20261106/A", ["c"]), ("puriri_cutlp_20261106", ["h1mv"]),
              ("tree_restart_guard_20261106", ["incc", "forcekc"]), ("submip_impr_20261106/C", ["fix", "fixh"]),
              ("ipmx_harm_20261106", ["ipmxm"]), ("mpsep_20261106", ["oq"]), ("rootlp_both_20261105", ["both"])]
    for d, cfgs in others:
        rows = []
        for f in sorted((AN / d).glob("bench58*.jsonl")):
            rr = bl.load([f])
            if any(r["rep"] in (None,) for r in rr) or all("rep" not in str(x) for x in [0]):
                pass
            for r in rr:
                if r["rep"] in (0, None) and "_s" in f.stem:
                    r["rep"] = f.stem
            rows += rr
        R = bl.table(rows)
        n58 = loo(groups58, NAMES58, "other:" + d, 12)
        for c in cfgs:
            print(f"\n--- {d} {c}")
            r58 = bl.report(R, "def", c, list(R), noise=n58, label=f"{d} {c}")
            table.append((d, c, r58, None))
    print("\n\n== まとめ (58 問。SGM の比、(b) 対の比 [95% 区間] p、(a1) 揺れの大きい 12 問を除く、(a2) 逆数の重み、(c) 片方だけ解けた回 A/B p、(d) 根の下界のずれ)")
    for b, c, r, rM in table:
        if not r:
            continue
        rg = r.get("root_gap")
        rgs = f"{rg[0]:+.3f} p={rg[3]:.2f} ({rg[5]}/{rg[4]})" if rg else "-"
        print(f"  {b:30s} {c:9s} SGM {r['sgmB'] / r['sgmA'] - 1:+5.1%} | (b) {summary_line(r, 'eq')} | (a1) {summary_line(r, 'drop')} | "
              f"(a2) {summary_line(r, 'inv')} | (c) {r['solved'][0]}/{r['solved'][1]} p={r['solved'][2]:.2f} | (d) {rgs}")
    print("\n== まとめ (Mittelmann 186 問、1 回)")
    for b, c, _, r in table:
        if not r:
            continue
        rg = r.get("root_gap")
        rgs = f"{rg[0]:+.3f} p={rg[3]:.2f} ({rg[5]}/{rg[4]})" if rg else "-"
        print(f"  {b:10s} {c:9s} SGM {r['sgmB'] / r['sgmA'] - 1:+5.1%} | (b) {summary_line(r, 'eq')} | (a1) {summary_line(r, 'drop')} | "
              f"(a2) {summary_line(r, 'inv')} | (c) {r['solved'][0]}/{r['solved'][1]} p={r['solved'][2]:.2f} | (d) {rgs}")


def paired_noise():
    """比べ (同じ鍵の def と試作) の差から見た 1 回あたりの分散の和 (真の差も含むので上限)。"""
    print("\n" + "=" * 100)
    print("1b. 比べの対の差から見た揺れ (Σ_p Δ²/2 を回の数で割った、1 回の分散の和に当たる量。真の差を含むので上限)")
    for lab, sel, ref in (("58 問", lambda k: k[1] < 100, sum(N58["var"].values())), ("Mittelmann", lambda k: k[1] >= 100, sum(NM["var"].values()))):
        vals = []
        for n, (_, cfgs) in HSB.items():
            R = bl.table(nz.hs_rows(n))
            for c in cfgs:
                pr = bl.paired(R, "def", c, [k for k in R if sel(k)])
                nrep = len({k[1] for k, *_ in pr})
                vals.append((sum(d * d for _, d, _, _ in pr) / 2 / nrep, f"bench{n} {c}"))
        vals.sort()
        q = [vals[int(len(vals) * f)][0] for f in (0.25, 0.5, 0.75)]
        print(f"  {lab}: {len(vals)} 比べ、最小 {vals[0][0]:.2f} ({vals[0][1]})・四分位 {q[0]:.2f} / {q[1]:.2f} / {q[2]:.2f}・最大 {vals[-1][0]:.2f} ({vals[-1][1]})"
              f"  (既定どうしの見積もり {ref:.2f})")


if __name__ == "__main__":
    part1()
    paired_noise()
    part2()
    part3()
    pooled_ref()
    print("\n== 既定の回の運 (正 = 既定の回が遅く、どの試作も速く見える)")
    for b, lab, m, pct, top in LUCK:
        print(f"  {b:8s} {lab:10s} {m:+.4f} (SGM 相当 {pct:+.1%})  " + ", ".join(f"{k[0]} r{k[1]} {x:+.2f}" for x, k in top))
