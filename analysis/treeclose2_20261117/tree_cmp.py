"""(2) ノードごとの記録の集計: python tree_cmp.py 問題 → 標準出力。
logs/問題_highs_cut_t1_tl.log.gz の HNODE・HBR と logs/問題_ours_cut_t1_tl.log.gz の ENODE・EBR を読み、並べる。
- 終わり方の内訳 (HiGHS: r = 0 下界・1 伝播で矛盾 (衝突のプールを含む)・2 LP 実行不能・3 分枝・4 打ち切り値超え (suboptimal)。
  当方: pop_lb・prop・lpinf・lpobj・redcost・sb・sbprop・int・tubr・sbinherit・分枝 (EBR))
- 刈ったノードの深さの分布、LP の伸び (ノードの LP 値 − 取り出したときの下界) の分布 (出どころ = 潜り / 待ち行列ごと)
- 全体の下界と開いたノードの時間ごとの値 (当方は `MIP: ... open ... lb` の行、HiGHS は表示の行)
HiGHS の inbr=1 の HNODE (分枝の中の強分岐の固定の後の解き直し) は同じノードなので数えない。"""
import gzip, re, statistics as st, sys
from collections import Counter, defaultdict
from pathlib import Path

D = Path(__file__).resolve().parent / "logs"
KV = re.compile(r"(\w+)=(\S+)")
HR = {0: "bound", 1: "prop", 2: "lpinf", 3: "branch", 4: "subopt", 5: "open"}


def parse(path, node_tag, br_tag):
    nodes, brs = [], []
    for line in gzip.open(path, "rt", errors="replace"):
        if line.startswith(node_tag + " "):
            nodes.append(dict(KV.findall(line)))
        elif line.startswith(br_tag + " "):
            brs.append(dict(KV.findall(line)))
    return nodes, brs


def q(xs, ps=(0.1, 0.5, 0.9)):
    if not xs:
        return "-"
    xs = sorted(xs)
    return " / ".join(f"{xs[min(len(xs) - 1, int(p * len(xs)))]:.4g}" for p in ps)


def fl(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return float("nan")


def summarize(name, rows, cutoff):
    """rows: (終わり方, 深さ, LP 値, 取り出したときの下界, 出どころ, 時刻)"""
    print(f"== {name}: ノード {len(rows)}")
    c = Counter(r[0] for r in rows)
    print("  終わり方: " + ", ".join(f"{k} {v} ({100 * v / len(rows):.1f}%)" for k, v in c.most_common()))
    src = Counter(r[4] for r in rows)
    print("  出どころ: " + ", ".join(f"{k} {v} ({100 * v / len(rows):.1f}%)" for k, v in src.most_common()))
    leaves = [r for r in rows if r[0] != "branch"]
    print(f"  葉 {len(leaves)}: 深さ (10% / 50% / 90%) {q([r[1] for r in leaves])}, 平均 {st.mean([r[1] for r in leaves]):.1f}" if leaves else "  葉 0")
    for k in ("bound", "lpobj", "pop_lb", "subopt", "prop", "lpinf", "redcost", "sb"):
        xs = [r[1] for r in leaves if r[0] == k]
        if xs:
            print(f"    {k:8s} {len(xs):6d}: 深さ {q(xs)}")
    br = [r for r in rows if r[0] == "branch"]
    print(f"  分枝したノードの深さ {q([r[1] for r in br])}")
    # LP の伸び: LP 値 − 取り出したときの下界 (どちらも有限のもの)、打ち切り値までの差との比も
    for s in sorted(src):
        g = [r[2] - r[3] for r in rows if r[4] == s and r[2] == r[2] and r[3] == r[3] and abs(r[2]) < 1e20 and abs(r[3]) < 1e20]
        gap = [(r[2] - r[3]) / max(1e-12, cutoff - r[3]) for r in rows if r[4] == s and r[2] == r[2] and r[3] == r[3] and abs(r[2]) < 1e20 and abs(r[3]) < 1e20 and cutoff - r[3] > 1e-9]
        if g:
            z = sum(1 for x in g if x <= 1e-9) / len(g)
            print(f"  LP の伸び ({s}, {len(g)}): 10% / 50% / 90% {q(g)}、伸び 0 の割合 {100 * z:.1f}%、打ち切り値までの差に対する比 {q(gap)}")


def lb_series(path, highs):
    out = []
    for line in gzip.open(path, "rt", errors="replace"):
        if highs:
            m = re.match(r"^\s*[A-Z]?\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)%\s+(\S+)\s+(\S+)\s+\S+\s+\d+\s+\d+\s+\d+\s+\S+\s+([\d.]+)s", line)
            if m:
                out.append((float(m.group(7)), int(m.group(1)), int(m.group(2)), fl(m.group(5)), float(m.group(4))))
        else:
            m = re.match(r"^MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+)", line)
            if m:
                out.append((float(m.group(1)), int(m.group(2)), int(m.group(3)), fl(m.group(4)), float("nan")))
    return out


def main():
    p = sys.argv[1]
    cutoff = {"csched008": 172.5, "sct2": -230.98916228571431}[p]
    hn, hb = parse(D / f"{p}_highs_cut_t1_tl.log.gz", "HNODE", "HBR")
    hrows = []
    for n in hn:
        if n.get("inbr") == "1":
            continue
        hrows.append((HR.get(int(n["r"]), n["r"]), int(n["d"]), fl(n["lp"]), fl(n["plb"]), n["src"], fl(n["t"])))
    on, ob = parse(D / f"{p}_ours_cut_t1_tl.log.gz", "ENODE", "EBR")
    orows = [(n["r"], int(n["d"]), fl(n.get("lp")), fl(n.get("plb")), n.get("src", "?"), fl(n.get("t"))) for n in on]
    orows += [("branch", int(b["d"]), fl(b.get("lp")), fl(b.get("plb")), b.get("src", "?"), fl(b.get("t"))) for b in ob]
    summarize("HiGHS", hrows, cutoff)
    summarize("当方", orows, cutoff)
    # 分枝の擬似費用 (選んだ列の down・up の積の分布)
    for name, bs in (("HiGHS", hb), ("当方", ob)):
        prod = [fl(b["pcd"]) * fl(b["pcu"]) for b in bs if "pcd" in b]
        print(f"== {name} の分枝 {len(bs)}: 選んだ列の擬似費用の積 {q(prod)}")
    # 全体の下界と開いたノードの推移
    for name, hi in (("HiGHS", True), ("当方", False)):
        f = D / f"{p}_{'highs' if hi else 'ours'}_cut_t1_tl.log.gz"
        ser = lb_series(f, hi)
        print(f"== {name} の推移 (秒, ノード, 開いた (HiGHS は待ち行列), 下界{', 木の探った割合 %' if hi else ''})")
        step = max(1, len(ser) // 15)
        for r in ser[::step] + ser[-1:]:
            print("   " + f"{r[0]:7.1f} {r[1]:7d} {r[2]:6d} {r[3]:.10g}" + (f" {r[4]:.2f}" if hi else ""))


if __name__ == "__main__":
    main()
