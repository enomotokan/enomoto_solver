"""run_long.py の結果 JSON と ENOMOTO のログから、表と推移 (木・RSS・ログの大きさ) を作る。

使い方: python analysis/mittelmann_ready_20261103/long_c/summarize.py [--tl 600] [--logdir /tmp/long_c_logs]
出力: 同じディレクトリの summary_<tl>.md (表と推移) と <name>_ours_<tl>_excerpt.log (ログの要点)
"""
import argparse, json, re
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROG = re.compile(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)")


def gap(obj, bd):
    if obj is None or bd is None:
        return None
    if abs(obj - bd) <= 1e-9 * max(1, abs(obj)):
        return 0.0
    return abs(obj - bd) / max(abs(obj), abs(bd), 1e-10)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tl", type=int, default=600)
    ap.add_argument("--logdir", type=Path, default=Path("/tmp/long_c_logs"))
    a = ap.parse_args()
    ours = json.loads((HERE / f"results_ours_{a.tl}.json").read_text())
    hp = HERE / f"results_highs_{a.tl}.json"
    highs = json.loads(hp.read_text()) if hp.exists() else {}
    md = [f"# 組 C 時間の上限 {a.tl} 秒・4 スレッドの集計 (summarize.py で生成)\n",
          "| 問題 | ENOMOTO 状態 | 時間 (s) | 目的値 | 下界 | ギャップ | ノード | 最大 RSS (MiB) | ログ (KB) | 警告 | HiGHS 状態 | 時間 (s) | 目的値 | 下界 | ギャップ | ノード | 最大 RSS (MiB) |",
          "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    f = lambda x, p=6: "-" if x is None else (f"{x:.{p}g}" if isinstance(x, float) else str(x))
    g = lambda x: "-" if x is None else f"{100*x:.4f}%"
    for n, o in ours.items():
        h = highs.get(n, {})
        md.append(f"| {n} | {o.get('status')} | {f(o.get('time'),4)} | {f(o.get('objective'),10)} | {f(o.get('bound'),10)} | {g(gap(o.get('objective'), o.get('bound')))} | "
                  f"{o.get('nodes')} | {o['maxrss_kib']/1024:.0f} | {o['log_bytes']/1024:.0f} | {o.get('mip_warnings') or 0} | "
                  f"{h.get('status','-')} | {f(h.get('time'),4)} | {f(h.get('objective'),10)} | {f(h.get('bound'),10)} | {g(gap(h.get('objective'), h.get('bound')))} | {h.get('nodes','-')} | "
                  + (f"{h['maxrss_kib']/1024:.0f} |" if h else "- |"))
    md.append("\n## 推移 (ENOMOTO: ログの進み具合の行を約 60 秒ごとに抜粋、RSS は run_long.py の 5 秒ごとの標本を 60 秒ごとに)\n")
    for n, o in ours.items():
        log = a.logdir / f"{n}_ours_{a.tl}.log"
        rows = []
        if log.exists():
            lines = log.read_text(errors="replace").splitlines()
            last = -1e9
            for l in lines:
                m = PROG.search(l)
                if m and float(m.group(1)) - last >= 60:
                    rows.append(m.groups()); last = float(m.group(1))
            prog = [l for l in lines if PROG.search(l)]
            if prog and (not rows or PROG.search(prog[-1]).groups() != rows[-1]):
                rows.append(PROG.search(prog[-1]).groups())
            # ログの要点: 進み具合以外の行のうち最初の 60 行と最後の 40 行 + 抜粋した進み具合
            keep = [l for l in lines if not PROG.search(l)]
            ex = keep[:60] + ["...", "(進み具合の抜粋)"] + [l for l in lines if PROG.search(l) and PROG.search(l).groups() in rows] + ["..."] + keep[-40:]
            (HERE / f"{n}_ours_{a.tl}_excerpt.log").write_text("\n".join(x[:400] for x in ex) + "\n")
        md.append(f"### {n}\n\n| 時刻 (s) | ノード | 開いたノード | lb | ub |\n|---|---|---|---|---|")
        md += [f"| {r[0]} | {r[1]} | {r[2]} | {r[3]} | {r[4]} |" for r in rows]
        md.append("\n| 経過 (s) | RSS (MiB) | ログ (KB) |\n|---|---|---|")
        last = -1e9
        ser = o["rss_series"]
        for t, rss, lb in ser:
            if t - last >= 60 or (t, rss, lb) == tuple(ser[-1]):
                md.append(f"| {t} | {rss/1024:.0f} | {lb/1024:.0f} |"); last = t
        if n in highs:
            hs = highs[n]["rss_series"]
            md.append("\nHiGHS の RSS (MiB): " + ", ".join(f"{t:.0f}s {r/1024:.0f}" for t, r, _ in hs[::12] + hs[-1:]))
        md.append("")
    (HERE / f"summary_{a.tl}.md").write_text("\n".join(md) + "\n")
    print("\n".join(md[:12]))


if __name__ == "__main__":
    main()
