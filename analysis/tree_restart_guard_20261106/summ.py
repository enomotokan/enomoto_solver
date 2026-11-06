"""ログ (logs/<タグ>_<問題>_<種>.log) から木の中の再スタートの時点の量を表にする。
使い方: python summ.py [問題 ...]   (res.jsonl の回ごとに 1 行 + 木の中の再スタートごとに 1 行)"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
KV = re.compile(r"(\w+) (\S+)")


def parse(path):
    """木の中の再スタートの一覧: 各要素は再スタートの時点の TRSTAT の値と、その後の求解の前処理・根の値。"""
    ev, cur, tstart, stops = [], None, None, 0
    lines = path.read_text(errors="replace").splitlines()
    for ln in lines:
        if ln.startswith("TRSTAT: tree start"):
            d = dict(KV.findall(ln[len("TRSTAT: tree start "):]))
            tstart = d
            if cur is not None and "next_root" not in cur:
                cur["next_root"] = float(d["solve"])
                cur["next_root_lb"] = float(d["lb"])
                cur = None if "root_end" in cur else cur
        elif ln.startswith("TRSTAT: due") or ln.startswith("TRSTAT: inc"):
            parts = ln.split()
            d = dict(re.findall(r"(\w+) ([^\s(]+)", ln[len("TRSTAT: inc "):]))
            if "guard pass" in ln:
                cur = {"why": parts[1], **{k: d[k] for k in ("outer", "root", "tree", "nodes", "open", "q", "est_left", "lb", "root_lb", "ub", "gap", "newly", "left")}}
                cur["frac"] = re.search(r"newly \S+ \(([\d.]+)\)", ln).group(1)
                ev.append(cur)
            else:
                stops += 1
        elif cur is not None:
            m = re.match(r"MIP: restart \d+ \(.*?(\d+) cuts kept\)", ln)
            if m and "kept" not in cur:
                cur["kept"] = int(m.group(1))
            m = re.match(r"MIP: presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+)", ln)
            if m and "kept" in cur and "rows" not in cur:
                cur["rows"] = f"{m.group(1)}→{m.group(2)}"
                cur["cols"] = f"{m.group(3)}→{m.group(4)}"
            m = re.match(r"MIP: cut loop done: .* obj (\S+) -> (\S+)", ln)
            if m and "kept" in cur and "root_lp" not in cur:
                cur["root_lp"], cur["root_end"] = float(m.group(1)), float(m.group(2))
    return ev, stops


def main():
    probs = set(sys.argv[1:])
    rows = [json.loads(l) for l in open(D / "res.jsonl")]
    for r in rows:
        if probs and r["problem"] not in probs:
            continue
        res = r["result"]
        log = D / f"logs/{r['tag']}_{r['problem']}_{r['seed']}.log"
        ev, stops = parse(log) if log.exists() else ([], 0)
        t = res.get("time") or 0
        print(f"| {r['problem']} | {r['tag']} | {r['seed']} | {res.get('status')} | {t:.1f} | {res.get('nodes')} | {len(ev)} | {stops} |")
        for i, e in enumerate(ev, 1):
            f = lambda k: e.get(k, "-")  # noqa: E731
            lb, rl, ub = float(e["lb"]), float(e["root_lb"]), float(e["ub"])
            closed = (lb - rl) / (ub - rl) if ub > rl else float("nan")
            nr = e.get("root_lp")
            drop = f"{nr - rl:+.4g}" if nr is not None else "-"
            dend = f"{e['root_end'] - rl:+.4g}" if "root_end" in e else "-"
            print(f"|  ↳ #{i} {e['why']} | t {float(e['outer']):.1f} | n {e['nodes']} open {e['open']} q {float(e['q']):.3f} | 根 {float(e['root']):.1f}s 木 {float(e['tree']):.1f}s 残り見込み {float(e['est_left']):.1f}s 残り {float(e['left']):.0f}s"
                  f" | gap {float(e['gap']):.2e} 閉じた {closed:.2f} | 固定 {e['newly']} ({float(e['frac']):.3f}) | 行 {f('rows')} 列 {f('cols')} カット {f('kept')}"
                  f" | 後の根 LP−前の根 {drop}, 根の終わり−前の根 {dend}, 根 {f('next_root')}s |")


if __name__ == "__main__":
    main()
