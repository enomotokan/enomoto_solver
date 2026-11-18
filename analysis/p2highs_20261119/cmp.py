"""P2-HIGHS の比べ: python cmp.py 問題 [設定 ...] → 標準出力 (既定の設定は highs_t1 と ours_t1)。
logs/問題_設定.log.gz から、前処理後のモデル・再スタート・根の下界・下界と開いたノードの推移・木の葉の深さと終わり方・
深さごとの固まった整数列を並べる (HiGHS は HNODE・HBR と表示の行、当方は ENODE・EBR と `MIP: ... nodes ... open ... lb` の行)。"""
import gzip, re, sys
from collections import Counter, defaultdict
from pathlib import Path

D = Path(__file__).resolve().parent / "logs"
KV = re.compile(r"(\w+)=(\S+)")
HR = {0: "bound", 1: "prop", 2: "lpinf", 3: "branch", 4: "subopt", 5: "branch"}


def q(xs, ps=(0.1, 0.5, 0.9)):
    xs = sorted(xs)
    return "/".join(f"{xs[min(len(xs) - 1, int(p * len(xs)))]:.4g}" for p in ps) if xs else "-"


def fl(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return float("nan")


def show(p, cfg):
    f = D / f"{p}_{cfg}.log.gz"
    if not f.exists():
        print(f"== {cfg}: ログなし"); return
    hi = cfg.startswith("highs")
    lines = gzip.open(f, "rt", errors="replace").read().splitlines()
    res = next((l for l in reversed(lines) if l.startswith("{")), "")
    print(f"== {cfg}: {res[:160]}")
    # 前処理・再スタート・対称性
    pat = (r"Presolved problem has|Model after restart|inactive integer columns|Found \d+ generator|Restarting search|Objective function|Solving root"
           if hi else r"presolve: rows|MIP_FOLD: n |restart \d|early restart|symmetry: \d+ generators|root LP|cuts: root|root bound|tree restart|TREER")
    for l in [l for l in lines if re.search(pat, l)][:14]:
        print("   ", l.strip()[:170])
    # 推移
    ser = []
    for l in lines:
        if hi:
            m = re.match(r"^\s*[A-Z]?\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)%\s+(\S+)\s+(\S+)\s+\S+\s+\d+\s+\d+\s+\d+\s+\S+\s+([\d.]+)s", l)
            if m:
                ser.append((float(m.group(7)), int(m.group(1)), int(m.group(2)), fl(m.group(5)), fl(m.group(6)), float(m.group(4))))
        else:
            m = re.match(r"^MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)", l)
            if m:
                ser.append((float(m.group(1)), int(m.group(2)), int(m.group(3)), fl(m.group(4)), fl(m.group(5)), float("nan")))
    step = max(1, len(ser) // 10)
    print("    推移 (秒, ノード, 開いた, 下界, 上界" + (", 探った %)" if hi else ")"))
    for r in ser[::step] + ser[-1:]:
        print(f"      {r[0]:7.1f} {r[1]:8d} {r[2]:7d} {r[3]:.10g} {r[4]:.10g}" + (f" {r[5]:.1f}" if hi else ""))
    # 木
    rows = []
    for l in lines:
        if l.startswith("HNODE ") or l.startswith("ENODE ") or l.startswith("EBR "):
            d = dict(KV.findall(l))
            if d.get("inbr") == "1":
                continue
            r = "branch" if l.startswith("EBR ") else (HR.get(int(d["r"]), d["r"]) if hi else d["r"])
            rows.append((r, int(d["d"]), int(d.get("nfix", -1)), d.get("src", "?"), fl(d.get("lp")) - fl(d.get("plb"))))
    if not rows:
        return
    leaves = [x for x in rows if x[0] != "branch"]
    print(f"    ノード {len(rows)}、葉 {len(leaves)} の深さ {q([x[1] for x in leaves])}、分枝したノードの深さ {q([x[1] for x in rows if x[0] == 'branch'])}")
    print("    葉の終わり方: " + ", ".join(f"{k} {v}" for k, v in Counter(x[0] for x in leaves).most_common(6)))
    g = [x[4] for x in rows if x[4] == x[4]]
    if g:
        print(f"    LP の伸び (LP − 取り出したときの下界) 10/50/90%: {q(g)}、0 の割合 {100 * sum(1 for v in g if abs(v) <= 1e-9) / len(g):.0f}%")
    by = defaultdict(list)
    for x in rows:
        if x[2] >= 0:
            by[x[1]].append(x[2])
    med = lambda xs: sorted(xs)[len(xs) // 2] if xs else None  # noqa: E731
    print("    深さ 0/10/20/40/80 の固まった整数列 (中央): " + " / ".join(str(med(by.get(k, []))) for k in (0, 10, 20, 40, 80)))


if __name__ == "__main__":
    p = sys.argv[1]
    for c in sys.argv[2:] or ["highs_t1", "ours_t1"]:
        show(p, c)
