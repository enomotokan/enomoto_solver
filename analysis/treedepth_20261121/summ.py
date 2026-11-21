"""診断 `ENOMOTO_MIP_TREEDEPTH_DIAG` の生の記録を深さ別に集計する (analysis/treedepth_20261121/README.md)。
使い方: python summ.py OUT.jsonl [スレッド数 [問題,...]]  → Markdown の表を標準出力に出す。
同じ問題・同じスレッド数の種はまとめて (スレッドもまとめて) 数える。"""
import gzip, json, sys
from collections import defaultdict
from pathlib import Path
import statistics as st
D = Path(__file__).resolve().parent
OUT = Path(sys.argv[1])
TH = int(sys.argv[2]) if len(sys.argv) > 2 else 4
ONLY = sys.argv[3].split(",") if len(sys.argv) > 3 else None
BK = [(1, 1), (2, 2), (3, 4), (5, 8), (9, 16), (17, 10**9)]
NAMES = ["1", "2", "3-4", "5-8", "9-16", "17+"]


def bk(d):
    for i, (a, b) in enumerate(BK):
        if a <= d <= b:
            return i
    return None


def q(v, p):
    if not v:
        return float("nan")
    v = sorted(v)
    return v[min(len(v) - 1, int(p * len(v)))]


def load(stem, p, th, seeds):
    recs = []
    for s in seeds:
        f = D / "logs" / f"{stem}_{p}_t{th}_s{s}.jsonl.gz"
        if f.exists():
            for l in gzip.open(f, "rt"):
                if l.strip():
                    recs.append(json.loads(l))
    return recs


def summarize(recs):
    B = [defaultdict(list) for _ in BK]
    C = [defaultdict(float) for _ in BK]
    tot_ticks = 0.0
    for r in recs:
        for d, n in enumerate(r["pop"]):
            i = bk(d)
            if i is not None:
                C[i]["pop"] += n
        for d, n in enumerate(r["all"]):
            i = bk(d)
            if i is not None:
                C[i]["all"] += n
        first = {}
        for x in r["rounds"]:
            d, seq, kind, g, n, tk, tk2, s1, s2 = x
            i = bk(d)
            if i is None:
                continue
            c = C[i]
            c["rounds"] += 1
            c["cuts"] += n
            c["ticks"] += tk + tk2
            c["secs"] += s1 + s2
            tot_ticks += tk + tk2
            if kind == 3:
                c["none"] += 1
            if seq not in first:
                first[seq] = kind
                c["nodes"] += 1
                c["node_none"] += kind == 3
            if kind in (0, 1):
                B[i]["gain"].append(g)
                c["cutoff"] += kind == 1
        for d, sub in r["nodes_sep"]:
            i = bk(d)
            if i is not None:
                B[i]["sub"].append(sub)
        for d, seq, lin, life, ba, bs, t0 in r["cuts"]:
            i = bk(d)
            if i is None:
                continue
            B[i]["lin"].append(lin)
            if life >= 0:
                B[i]["life"].append(life)
            else:
                C[i]["alive"] += 1
            C[i]["ncut"] += 1
            C[i]["bound"] += ba > 0
            C[i]["bound_sub"] += bs > 0
            C[i]["bind"] += ba
            C[i]["bind_sub"] += bs
            C[i]["lin_sum"] += lin
    return B, C, tot_ticks


def table(p, recs, nruns):
    B, C, T = summarize(recs)
    print(f"\n#### {p} ({nruns} 回, スレッド {TH})\n")
    print("| 深さ | 取り出し | 分離ノード (率) | ラウンド | 見つからず (ノード) | カット | 手間 % | 秒 | 伸び 中央 / 上位10% | 打ち切り | 部分木 LP 中央 | 寿命 (LP にあった回) 中央 / 上位10% | 張った本の率 (部分木) | 張った回/本 (部分木) | 張った回/LP にあった回 | 伸びの和/1e9 手間 |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for i, nm in enumerate(NAMES):
        c, b = C[i], B[i]
        if c["pop"] == 0 and c["rounds"] == 0:
            continue
        g = b["gain"]
        lf = b["lin"]
        print(
            f"| {nm} | {int(c['pop'])} | {int(c['nodes'])} ({c['nodes'] / max(c['pop'], 1):.2f}) | {int(c['rounds'])} | "
            f"{c['none'] / max(c['rounds'], 1):.2f} ({c['node_none'] / max(c['nodes'], 1):.2f}) | {int(c['cuts'])} | "
            f"{100 * c['ticks'] / max(T, 1):.0f} | {c['secs']:.1f} | "
            + (f"{q(g, 0.5):.2g} / {q(g, 0.9):.2g}" if g else "-")
            + f" | {c['cutoff'] / max(len(g), 1):.2f} | {q(b['sub'], 0.5):.0f} | "
            + (f"{q(lf, 0.5):.0f} / {q(lf, 0.9):.0f}" if lf else "-")
            + f" | {c['bound'] / max(c['ncut'], 1):.2f} ({c['bound_sub'] / max(c['ncut'], 1):.2f}) | {c['bind'] / max(c['ncut'], 1):.1f} ({c['bind_sub'] / max(c['ncut'], 1):.1f}) | {c['bind'] / max(c['lin_sum'], 1):.3f} | {sum(g) / max(c['ticks'] / 1e9, 1e-9):.2f} |"
        )
    return B, C, T


if __name__ == "__main__":
    rows = [json.loads(l) for l in open(OUT) if l.strip()]
    probs = []
    for r in rows:
        if r["threads"] == TH and r["problem"] not in probs:
            probs.append(r["problem"])
    if ONLY:
        probs = [p for p in probs if p in ONLY]
    for p in probs:
        rr = [r for r in rows if r["problem"] == p and r["threads"] == TH]
        seeds = [r["rep"] for r in rr]
        res = "; ".join(f"s{r['rep']} {r['result'].get('status')} {r['result'].get('time') or 0:.0f}s ノード {r['result'].get('nodes')} 下界 {r['result'].get('bound')}" for r in rr)
        recs = load(OUT.stem, p, TH, seeds)
        table(p, recs, len(rr))
        print(f"\n結果: {res}")


ROOTLP = {}
if (D / "rootlp.jsonl").exists():
    for l in open(D / "rootlp.jsonl"):
        if l.strip():
            x = json.loads(l)
            ROOTLP[x["problem"]] = x["root_lp"][0]


def rootinfo(recs, p=None):
    """木の始めの値: (暫定解の有無, 相対ギャップ, 根で閉じた割合, 根のカットの手間 (1e9), 木の始めまでの手間 (1e9))。値は最小化形。
    最初の根の LP の値 (rootlp.py が拾ったもの、カット・再スタートの前) があれば、根で閉じた割合は
    (木の始めの下界 − 最初の根の LP) / (最良の解 − 最初の根の LP)、手間は木の始めまでの手間 (前処理・ヒューリスティック・再スタートを含む) にする"""
    # 根のカットの記録は主スレッド (tid 0) の最後の求解のものを使う。再スタートがあれば、その後の根の値になる
    main = [r for r in recs if r.get("root") and r["tid"] == 0]
    if not main:
        return None
    m = main[-1]
    o0 = m["root"][0][0]
    o1 = m["root"][-1][1]
    rt = sum(x[2] for x in m["root"]) / 1e9
    inc, lb, tk, _ = m["start"] if m.get("start") else (None, None, 0, 0)
    fins = [r["fin"][0] for r in recs if r.get("fin") and r["fin"][0] is not None]
    best = min(fins) if fins else None
    gap = abs(inc - lb) / max(abs(inc), 1) if inc is not None and lb is not None else None
    closed = (o1 - o0) / (best - o0) if best is not None and best - o0 > 1e-9 else None
    if p in ROOTLP and lb is not None:
        r0 = ROOTLP[p]
        closed = (lb - r0) / (best - r0) if best is not None and best - r0 > 1e-9 else None
        rt = tk / 1e9
    return inc is not None, gap, closed, rt, tk / 1e9


def cross(stem, th, rows, probs):
    print(f"\n### 問題をまたいだ並び (スレッド {th}、種をまとめる)\n")
    print("| 問題 | 始めの暫定解 | 始めのギャップ | 根で閉じた割合 | 木の始めまでの手間 (1e9) | 閉じた割合/1e9 手間 | 木の分離の秒の割合 | 1 ラウンドの伸び 中央 (深さ 1-4 / 9+) | 見つからない率 (1-4 / 9+) | 張った回/LP にあった回 (1-4 / 9+) | 伸びの和/1e9 手間 (1-4 / 9+) | 木の分離の手間のうち深さ 1-4 の割合 | 寿命 中央 (1-4 / 9+) | 張った回/本 (1-4 / 9+) |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for p in probs:
        rr = [r for r in rows if r["problem"] == p and r["threads"] == th]
        per = []
        for r in rr:
            recs = load(stem, p, th, [r["rep"]])
            per.append((rootinfo(recs, p), recs))
        ri = [x[0] for x in per if x[0]]
        recs = [x for _, rs in per for x in rs]
        B, C, T = summarize(recs)
        def agg(lo, hi):
            g, c = [], defaultdict(float)
            for i in range(lo, hi):
                g += B[i]["gain"]
                for k, v in C[i].items():
                    c[k] += v
            return g, c
        sh, ch = agg(0, 3)
        dp, cd = agg(4, 6)
        lsh = [v for i in range(0, 3) for v in B[i]["lin"]]
        ldp = [v for i in range(4, 6) for v in B[i]["lin"]]
        secs = sum(C[i]["secs"] for i in range(6))
        wall = sum(min(r["result"].get("time") or r["tl"], r["tl"]) for r in rr) * th
        f = lambda v, fmt: (fmt.format(v) if v is not None else "-")
        print(
            f"| {p} | {'/'.join('有' if x[0] else '無' for x in ri)} | {'/'.join(f(x[1], '{:.2g}') for x in ri)} | {'/'.join(f(x[2], '{:.2f}') for x in ri)} | "
            f"{'/'.join(f'{x[3]:.2g}' for x in ri)} | {'/'.join(f(x[2] / x[3] if x[2] is not None and x[3] > 0 else None, '{:.2g}') for x in ri)} | {secs / max(wall, 1e-9):.3f} | "
            f"{q(sh, 0.5):.2g} / {q(dp, 0.5):.2g} | {ch['none'] / max(ch['rounds'], 1):.2f} / {cd['none'] / max(cd['rounds'], 1):.2f} | "
            f"{ch['bind'] / max(ch['lin_sum'], 1):.3f} / {cd['bind'] / max(cd['lin_sum'], 1):.3f} | "
            f"{sum(sh) / max(ch['ticks'] / 1e9, 1e-9):.2f} / {sum(dp) / max(cd['ticks'] / 1e9, 1e-9):.2f} | "
            f"{ch['ticks'] / max(T, 1):.2f} | {q(lsh, 0.5):.0f} / {q(ldp, 0.5):.0f} | {ch['bind'] / max(ch['ncut'], 1):.1f} / {cd['bind'] / max(cd['ncut'], 1):.1f} |"
        )


if __name__ == "__main__":
    cross(OUT.stem, TH, rows, probs)
