"""58 問・Mittelmann の 60 秒の計測の共通の集計 (analysis/bench_method_20261106.md)。

使い方 (例: summ29.py の末尾に足す):

    import sys; sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    import benchlib as bl
    R = bl.table(bl.load(here.glob("bench29_s*.jsonl")))
    keys58 = [k for k in R if k[1] != 118]
    bl.report(R, "def", "s1", keys58, noise=bl.load_noise())

時間の扱いは今の summ*.py と同じ: 状態が optimal でかつ 60 秒以内なら解けた、それ以外は 60 秒。SGM のずらしは 10 秒。

中身:
- load / table: jsonl を読んで R[(問題, 回)][設定] = result にする。
- sgm, ok, t: 今の集計と同じ定義。
- paired: 同じ (問題, 回) の A と B の対の log((tB+10)/(tA+10))。その平均 Δ は「(SGM_B+10)/(SGM_A+10) の log」に一致する。
- ratio_ci: 問題ごとに回を平均した Δ_p を問題の単位で再抽出するブートストラップの信頼区間 (重みつきもできる)。
- signflip_p: Δ_p の符号をでたらめに反転させる並べ替え検定の p 値 (重みつきの平均に対して)。
- solved_test: 片方だけ解けた回の数の符号検定 (正確な 2 項)。
- root_cmp: 根の下界 (最良の既知値とのずれ)・初めの解の時刻・時間切れの回の最終ギャップの対の比べ。
- weights_for: (a) の重み。"drop" は揺れの大きい問題を除く、"inv" は感度/揺れ (sens_scale で全問の SGM の % に直す)。
- noise_*: 既定どうしの回から、問題ごとの 1 回の log(t+10) の揺れ (分散) を見積もる表 (bench_method_20261106/noise58.json などに保存)。
"""
from __future__ import annotations

import json
import math
import random
from collections import defaultdict
from pathlib import Path

TL = 60.0
SHIFT = 10.0
HERE = Path(__file__).resolve().parent
NOISE_DIR = HERE / "bench_method_20261106"


# ---------------------------------------------------------------- 読み込み

def load(paths, cfg_map=None, rep=None, src=None):
    """jsonl を読んで行の並びを返す。rep が無い行は rep 引数 (または seed、無ければ 0) を回とする。
    cfg_map で設定の名前を付け替えられる (例: {"base": "def"})。"""
    out = []
    for p in paths:
        p = Path(p)
        for line in open(p):
            line = line.strip()
            if not line:
                continue
            try:
                r = json.loads(line)
            except json.JSONDecodeError:
                continue
            if "result" not in r or "problem" not in r:
                continue
            c = r.get("cfg", r.get("config"))
            if cfg_map:
                c = cfg_map.get(c, c)
            rp = r.get("rep")
            if rp is None:
                rp = rep if rep is not None else r.get("seed", 0)
            out.append({"problem": r["problem"], "cfg": c, "rep": rp, "result": r["result"],
                        "src": src or str(p)})
    return out


def table(rows):
    """R[(問題, 回)][設定] = result。同じ鍵が二度あれば後のものを使う。"""
    R = defaultdict(dict)
    for r in rows:
        R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
    return R


# ---------------------------------------------------------------- 基本の量

def ok(res, tl=TL):
    return res.get("status") == "optimal" and res.get("time", 1e9) <= tl


def t(res, tl=TL):
    return res["time"] if ok(res, tl) else tl


def lt(res, tl=TL):
    """log(t + 10)。揺れの見積もり・対の比べはこの尺度で行う。"""
    return math.log(t(res, tl) + SHIFT)


def sgm(xs, shift=SHIFT):
    return math.exp(sum(math.log(x + shift) for x in xs) / len(xs)) - shift


def wsgm(xs, ws, shift=SHIFT):
    """重みつきの SGM (重みの和で割る)。"""
    sw = sum(ws)
    return math.exp(sum(w * math.log(x + shift) for x, w in zip(xs, ws)) / sw) - shift


def sgm_pct(sgm_a, delta_log, shift=SHIFT):
    """A の SGM と log((SGM_B+10)/(SGM_A+10)) から、SGM の比 (B/A − 1) を返す。"""
    return ((sgm_a + shift) * math.exp(delta_log) - shift) / sgm_a - 1


def log_from_pct(sgm_a, pct, shift=SHIFT):
    """SGM の比 pct (例 0.02) を log((SGM_B+10)/(SGM_A+10)) に直す。"""
    return math.log((sgm_a * (1 + pct) + shift) / (sgm_a + shift))


# ---------------------------------------------------------------- 対の比べ

def paired(R, A, B, keys=None):
    """[(鍵, ΔlogT, okA, okB)]。鍵は両方の設定がそろっているものだけ。"""
    keys = keys if keys is not None else list(R)
    out = []
    for k in keys:
        d = R.get(k, {})
        if A in d and B in d:
            out.append((k, lt(d[B]) - lt(d[A]), ok(d[A]), ok(d[B])))
    return out


def per_problem(pairs):
    """問題ごとに回を平均した Δ_p と回の数。"""
    acc = defaultdict(list)
    for k, dl, _, _ in pairs:
        acc[k[0]].append(dl)
    return {p: (sum(v) / len(v), len(v)) for p, v in acc.items()}


def _wmean(vals, ws):
    sw = sum(ws)
    return sum(v * w for v, w in zip(vals, ws)) / sw


def weights_for(probs, noise, kind="inv", floor=0.002):
    """問題ごとの重み。noise は load_noise() の辞書 ({"var": σ²_p, "sens": s_p, "noisy": [...]})。
    kind="inv": s_p / (σ²_p + floor)。s_p = 既定で解けた回の t/(t+10) の平均 (時間切れの回は 0) は「全問の時間が同じ割合 r だけ
      変わる」としたときの Δ_p ≈ r s_p の感度で、この重みは r を最も小さい揺れで推める (いつも時間切れ・0.1 秒の問題は重み ~0)。
    kind="drop": 揺れの大きい問題 (noise の noisy) を 0。"eq": 等しく。表に無い問題は揺れ・感度の中央値とみなす。"""
    if kind == "eq" or noise is None:
        return [1.0] * len(probs)
    if kind == "drop":
        hi = set(noise.get("noisy", []))
        return [0.0 if p in hi else 1.0 for p in probs]
    var, sens = noise["var"], noise.get("sens", {})
    mv = sorted(var.values())[len(var) // 2] if var else 0.01
    ms = sorted(sens.values())[len(sens) // 2] if sens else 0.5
    return [sens.get(p, ms) / (var.get(p, mv) + floor) for p in probs]


def sens_scale(probs, ws, noise):
    """重み ws の平均の Δ を、全問の SGM の log の差に直す倍率 (= 全問の s の平均 / 重みつきの s の平均)。
    共通の割合 r のモデルで Σ w Δ / Σ w ≈ r Σ w s / Σ w、全問の SGM の log の差 ≈ r mean(s)。"""
    sens = noise.get("sens", {}) if noise else {}
    if not sens:
        return 1.0
    ms = sorted(sens.values())[len(sens) // 2]
    s = [sens.get(p, ms) for p in probs]
    sw = sum(ws)
    ws_mean = sum(w * x for w, x in zip(ws, s)) / sw if sw else 0
    return (sum(s) / len(s)) / ws_mean if ws_mean > 0 else float("nan")


def ratio_ci(pp, ws=None, nboot=4000, alpha=0.05, seed=1):
    """Δ_p (per_problem の値) の重みつき平均と、問題の単位のブートストラップの百分位の区間 (log の尺度)。"""
    probs = sorted(pp)
    vals = [pp[p][0] for p in probs]
    ws = ws if ws is not None else [1.0] * len(probs)
    use = [(v, w) for v, w in zip(vals, ws) if w > 0]
    vals, ws = [u[0] for u in use], [u[1] for u in use]
    n = len(vals)
    if n == 0:
        return float("nan"), (float("nan"), float("nan"))
    m = _wmean(vals, ws)
    rng = random.Random(seed)
    bs = []
    for _ in range(nboot):
        idx = [rng.randrange(n) for _ in range(n)]
        sw = sum(ws[i] for i in idx)
        if sw <= 0:
            continue
        bs.append(sum(vals[i] * ws[i] for i in idx) / sw)
    bs.sort()
    lo = bs[int(alpha / 2 * len(bs))]
    hi = bs[min(len(bs) - 1, int((1 - alpha / 2) * len(bs)))]
    return m, (lo, hi)


def signflip_p(pp, ws=None, nperm=20000, seed=2):
    """Δ_p の符号をでたらめに反転させる並べ替え検定 (両側)。帰無仮説: 各問題で A と B は入れ替えても同じ分布。"""
    probs = sorted(pp)
    vals = [pp[p][0] for p in probs]
    ws = ws if ws is not None else [1.0] * len(probs)
    use = [(v, w) for v, w in zip(vals, ws) if w > 0 and v != 0.0]
    if not use:
        return 1.0
    sw = sum(w for _, w in use)
    obs = abs(sum(v * w for v, w in use)) / sw
    rng = random.Random(seed)
    cnt = 0
    for _ in range(nperm):
        s = sum(v * w if rng.random() < 0.5 else -v * w for v, w in use) / sw
        if abs(s) >= obs - 1e-15:
            cnt += 1
    return (cnt + 1) / (nperm + 1)


def consistent(pairs, thr=0.3, min_reps=2):
    """すべての回で同じ向きに |Δ| > thr の問題 (thr=0.3 は t+10 で約 35%。2 回・58 問なら揺れだけで約 0.9 問、0.15 なら約 3.3 問出る) (重みづけで薄まる「揺れの大きい問題の本物の差」を見落とさないための一覧)。
    返り値 [(問題, 平均 Δ, 回の数)]。"""
    acc = defaultdict(list)
    for k, d, _, _ in pairs:
        acc[k[0]].append(d)
    out = []
    for p, v in acc.items():
        if len(v) >= min_reps and (all(x > thr for x in v) or all(x < -thr for x in v)):
            out.append((p, sum(v) / len(v), len(v)))
    return sorted(out, key=lambda z: -abs(z[1]))


def binom_two_sided(k, n):
    """正確な 2 項の両側 p 値 (p=1/2)。"""
    if n == 0:
        return 1.0
    pk = [math.comb(n, i) / 2 ** n for i in range(n + 1)]
    return min(1.0, sum(x for x in pk if x <= pk[k] + 1e-15))


def solved_test(pairs):
    """片方だけ解けた回: (A だけ, B だけ, 2 項の p 値)。"""
    a = sum(1 for _, _, oa, ob in pairs if oa and not ob)
    b = sum(1 for _, _, oa, ob in pairs if ob and not oa)
    return a, b, binom_two_sided(min(a, b), a + b)


def problem_sign_test(pp, thr=0.05):
    """問題ごとの Δ_p が thr (log、約 5%) を越えて B が遅い/速い問題の数と 2 項の p 値。"""
    slow = sum(1 for v, _ in pp.values() if v > thr)
    fast = sum(1 for v, _ in pp.values() if v < -thr)
    return fast, slow, binom_two_sided(min(fast, slow), fast + slow)


# ---------------------------------------------------------------- 根の量

def best_known(R):
    """問題ごとの最良の既知の目的値 (どれかの回で optimal と返った値の中央値。無ければ最良の暫定解)。向きは root_bound と比べて推定する。"""
    opt, inc, rb = defaultdict(list), defaultdict(list), defaultdict(list)
    for (p, _), d in R.items():
        for res in d.values():
            if res.get("status") == "optimal" and res.get("objective") is not None:
                opt[p].append(res["objective"])
            elif res.get("objective") is not None:
                inc[p].append(res["objective"])
            if res.get("root_bound") is not None and math.isfinite(res["root_bound"]):
                rb[p].append(res["root_bound"])
    out = {}
    for p in set(opt) | set(inc):
        if opt[p]:
            v = sorted(opt[p])[len(opt[p]) // 2]
        else:
            # 最小化の見込み (根の下界 <= 暫定解) なら最小、そうでなければ最大
            if rb[p] and inc[p] and min(rb[p]) <= min(inc[p]):
                v = min(inc[p])
            else:
                v = max(inc[p]) if inc[p] else None
        if v is not None:
            out[p] = v
    return out


def rel_gap(a, b):
    if a is None or b is None or not (math.isfinite(a) and math.isfinite(b)):
        return None
    return abs(a - b) / max(1.0, abs(a), abs(b))


def root_cmp(R, A, B, keys=None, ref=None):
    """揺れの小さい量の対の比べ。返り値は辞書:
    root_gap: 根の下界と最良の既知値の相対のずれ (A,B の log10 の差、ずれ 1e-6 で切る) の Δ_p の並び
    first_sol: 初めの解の時刻の log((t+1)) の差の Δ_p
    final_gap: 両方とも時間切れの回の最終ギャップ (log10、1e-4 で切り、解なしは 1) の差の Δ_p"""
    ref = ref or best_known(R)
    keys = keys if keys is not None else list(R)
    acc = {"root_gap": defaultdict(list), "first_sol": defaultdict(list), "final_gap": defaultdict(list)}
    for k in keys:
        d = R.get(k, {})
        if A not in d or B not in d:
            continue
        p = k[0]
        a, b = d[A], d[B]
        if p in ref:
            ga, gb = rel_gap(a.get("root_bound"), ref[p]), rel_gap(b.get("root_bound"), ref[p])
            if ga is not None and gb is not None:
                acc["root_gap"][p].append(math.log10(max(gb, 1e-6)) - math.log10(max(ga, 1e-6)))
        fa, fb = a.get("first_solution_time"), b.get("first_solution_time")
        if fa is not None and fb is not None:
            acc["first_sol"][p].append(math.log(fb + 1) - math.log(fa + 1))
        if not ok(a) and not ok(b):
            def fg(r):
                g = rel_gap(r.get("objective"), r.get("bound"))
                return 0.0 if g is None else math.log10(max(g, 1e-4))
            acc["final_gap"][p].append(fg(b) - fg(a))
    return {name: {p: (sum(v) / len(v), len(v)) for p, v in m.items()} for name, m in acc.items()}


# ---------------------------------------------------------------- 揺れの表

def noise_from_groups(groups):
    """groups: [{問題: [log(t+10) の並び]}] (同じ既定の回の組ごと)。組の中の分散を自由度で重みつけて合わせ、
    問題ごとの 1 回の分散 σ²_p と自由度、解けた割合を返す。"""
    ss, df = defaultdict(float), defaultdict(int)
    for g in groups:
        for p, xs in g.items():
            if len(xs) < 2:
                continue
            m = sum(xs) / len(xs)
            ss[p] += sum((x - m) ** 2 for x in xs)
            df[p] += len(xs) - 1
    return {p: (ss[p] / df[p], df[p]) for p in ss if df[p] > 0}


def load_noise(name="noise58.json"):
    """bench_method_20261106/ に保存した揺れの表 ({"var": {問題: σ²}, "noisy": [...], ...})。"""
    f = NOISE_DIR / name
    if not f.exists():
        return None
    return json.load(open(f))


# ---------------------------------------------------------------- まとめの出力

def fmt_pct(sa, dl):
    return f"{sgm_pct(sa, dl):+.1%}"


def report(R, A, B, keys, noise=None, label="", nboot=4000, show_root=True, out=print):
    """1 つの比べ (A=既定、B=試作) を新しい集計で出す。返り値は主な数の辞書。"""
    keys = [k for k in keys if A in R.get(k, {}) and B in R.get(k, {})]
    pairs = paired(R, A, B, keys)
    if not pairs:
        out(f"{label}: 対がない")
        return {}
    sa = sgm([t(R[k][A]) for k in keys])
    sb = sgm([t(R[k][B]) for k in keys])
    na = sum(ok(R[k][A]) for k in keys)
    nb = sum(ok(R[k][B]) for k in keys)
    pp = per_problem(pairs)
    probs = sorted(pp)
    res = {"label": label, "n": len(keys), "sgmA": sa, "sgmB": sb, "solvedA": na, "solvedB": nb}
    out(f"[{label}] {A} 対 {B}: 鍵 {len(keys)} (問題 {len(probs)})  SGM {sa:.2f} → {sb:.2f} ({sb / sa - 1:+.1%})  解けた {na} → {nb}")
    # (b) 対の比 (等しい重み)
    m, (lo, hi) = ratio_ci(pp, nboot=nboot)
    p_eq = signflip_p(pp)
    res.update(eq=(m, lo, hi, p_eq))
    out(f"  (b) 対の比 (問題で回を平均、等しい重み): {fmt_pct(sa, m)}  95% 区間 [{fmt_pct(sa, lo)}, {fmt_pct(sa, hi)}]  符号反転の p={p_eq:.3f}")
    if noise is not None:
        # (a) 揺れの大きい問題を除く / 重みを下げる
        wd = weights_for(probs, noise, "drop")
        m2, (lo2, hi2) = ratio_ci(pp, wd, nboot=nboot)
        p2 = signflip_p(pp, wd)
        kept = [k for k in keys if k[0] not in set(noise.get("noisy", []))]
        s_a2 = sgm([t(R[k][A]) for k in kept]) if kept else float("nan")
        s_b2 = sgm([t(R[k][B]) for k in kept]) if kept else float("nan")
        res.update(drop=(m2, lo2, hi2, p2, s_a2, s_b2))
        out(f"  (a1) 揺れの大きい {sum(1 for w in wd if w == 0)} 問を除く: SGM {s_a2:.2f} → {s_b2:.2f} ({s_b2 / s_a2 - 1:+.1%})  "
            f"区間 [{fmt_pct(s_a2, lo2)}, {fmt_pct(s_a2, hi2)}]  p={p2:.3f}")
        wi = weights_for(probs, noise, "inv")
        sc = sens_scale(probs, wi, noise)
        m3, (lo3, hi3) = ratio_ci(pp, wi, nboot=nboot)
        p3 = signflip_p(pp, wi)
        res.update(inv=(m3 * sc, lo3 * sc, hi3 * sc, p3))
        out(f"  (a2) 揺れと感度の重み (全問の SGM に直した値): {fmt_pct(sa, m3 * sc)}  区間 [{fmt_pct(sa, lo3 * sc)}, {fmt_pct(sa, hi3 * sc)}]  p={p3:.3f}")
    cons = consistent(pairs)
    if cons:
        out("  揃った差 (2 回以上のすべての回で同じ向きに 0.3 (log) 超え、+ は遅い。2 回なら揺れだけで 58 問中 約 0.9 問出る): "
            + ", ".join(f"{p} {m:+.2f}×{n}" for p, m, n in cons))
    res["consistent"] = cons
    a, b, ps = solved_test(pairs)
    f_, s_, pps = problem_sign_test(pp)
    res.update(solved=(a, b, ps), psign=(f_, s_, pps))
    out(f"  (c) 片方だけ解けた回: {A} だけ {a}・{B} だけ {b} (2 項 p={ps:.3f})   問題ごと 5% 超: 速い {f_}・遅い {s_} (p={pps:.3f})")
    if show_root:
        rc = root_cmp(R, A, B, keys)
        for name, lab in (("root_gap", "根の下界のずれ (log10)"), ("first_sol", "初めの解の時刻 (log)"), ("final_gap", "時間切れの最終ギャップ (log10)")):
            d = rc[name]
            if len(d) < 3:
                continue
            mm, (l4, h4) = ratio_ci(d, nboot=nboot)
            p4 = signflip_p(d)
            nz = sum(1 for v, _ in d.values() if abs(v) > 1e-9)
            res[name] = (mm, l4, h4, p4, len(d), nz)
            out(f"  (d) {lab}: 平均 {mm:+.3f} [{l4:+.3f}, {h4:+.3f}] p={p4:.3f} (問題 {len(d)}、差のある {nz})")
    return res


# ---------------------------------------------------------------- 既定の回の運・既定を合わせた参照

def def_luck(R, A, cfgs, keys=None):
    """同じ bench で A (既定) と比べた試作がいくつもあるとき、A の回が試作の中央値からどれだけずれたか (log、問題で平均)。
    正なら既定の回が遅かった (どの試作も同じだけ速く見える)。返り値 (平均, [(ずれ, 鍵)] の大きい順)。"""
    import statistics as st
    keys = keys if keys is not None else list(R)
    out = []
    for k in keys:
        d = R.get(k, {})
        if A not in d or not all(c in d for c in cfgs):
            continue
        out.append((lt(d[A]) - st.median(lt(d[c]) for c in cfgs), k))
    if not out:
        return float("nan"), []
    # 問題で平均
    pp = defaultdict(list)
    for x, k in out:
        pp[k[0]].append(x)
    m = sum(sum(v) / len(v) for v in pp.values()) / len(pp)
    return m, sorted(out, key=lambda z: -abs(z[0]))


def pooled_cmp(trial, ref):
    """対にしない比べ: trial・ref は {問題: [log(t+10)]}。Δ_p = 平均(trial) − 平均(ref) を per_problem と同じ形で返す。
    同じ既定の別の bench の def を ref に合わせると、既定の 1 回の運を薄められる (機械の差は入る)。"""
    out = {}
    for p, xs in trial.items():
        ys = ref.get(p)
        if xs and ys:
            out[p] = (sum(xs) / len(xs) - sum(ys) / len(ys), len(xs))
    return out
