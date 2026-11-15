"""res.jsonl とログ (logs/) から設定ごとの結果と LP の反復を集計する。
使い方: python summ.py [設定,...]
各問題・設定で: 解けた回数 / 回数、時間 (解けない回は 60 秒) の平均、ノード数の平均、
木のノードの最初の LP の 1 回あたりの反復 (基底を復元した直後 / それ以外 = 潜りの子など)、
サブ MIP の根の LP・裏のカットの写しの根の LP・再スタートの後の根の LP の 1 回あたりの反復、
置き場所の希望で推し量りから変えた数 (1 求解あたり)。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
PAT = re.compile(r"basis nb diag: tree first LPs restored (\d+) iters (\d+) .*?others (\d+) iters (\d+) .*?root LPs: (.*); nb hints applied (\d+) solves, (\d+) given, (\d+) changed")
ROOT = re.compile(r"(\w+) (\d+) lps (\d+) iters")


def diag(log: Path):
    if not log.exists():
        return None
    m = None
    for line in log.read_text(errors="replace").splitlines():
        mm = PAT.search(line)
        if mm:
            m = mm  # 最後の行 (再スタートがあればその後のもの。数はプロセス全体の累計)
    if not m:
        return None
    g = list(map(int, [m.group(i) for i in (1, 2, 3, 4, 6, 7, 8)]))
    roots = {k: (int(a), int(b)) for k, a, b in ROOT.findall(m.group(5))}
    return {"rn": g[0], "ri": g[1], "on": g[2], "oi": g[3], "hs": g[4], "hg": g[5], "hc": g[6], "roots": roots}


def main():
    want = sys.argv[1].split(",") if len(sys.argv) > 1 else None
    recs = [json.loads(l) for l in (D / "res.jsonl").read_text().splitlines() if l.strip()]
    by = defaultdict(list)
    probs, cfgs = [], []
    for r in recs:
        if want and r["config"] not in want:
            continue
        by[(r["problem"], r["config"])].append(r)
        if r["problem"] not in probs:
            probs.append(r["problem"])
        if r["config"] not in cfgs:
            cfgs.append(r["config"])
    if want:
        cfgs = [c for c in want if c in cfgs]
    print("| 問題 | 設定 | 解けた | 時間 (平均) | ノード (平均) | 復元直後の LP 反復/回 | 他のノード LP 反復/回 | サブ MIP 根 反復/回 | 写しの根 反復/回 | 再スタート根 反復/回 | 希望で変えた数/求解 |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for p in probs:
        for c in cfgs:
            rs = by.get((p, c), [])
            if not rs:
                continue
            ok = sum(r["status"] == "optimal" for r in rs)
            t = sum(r["time"] if r["status"] == "optimal" and r["time"] is not None else 60.0 for r in rs) / len(rs)
            nodes = sum((r["nodes"] or 0) for r in rs) / len(rs)
            ds = [diag(D / "logs" / f"{p}_{c}_{r['rep']}.log") for r in rs]
            ds = [d for d in ds if d]

            def per(a, b, ds=ds):
                n = sum(d[a] for d in ds)
                return f"{sum(d[b] for d in ds) / n:.1f}" if n else "-"

            def rper(k, ds=ds):
                n = sum(d["roots"].get(k, (0, 0))[0] for d in ds)
                return f"{sum(d['roots'].get(k, (0, 0))[1] for d in ds) / n:.1f} ({n})" if n else "-"

            hc = sum(d["hc"] for d in ds)
            hs = sum(d["hs"] for d in ds)
            print(f"| {p} | {c} | {ok}/{len(rs)} | {t:.1f} | {nodes:.0f} | {per('rn', 'ri')} | {per('on', 'oi')} | {rper('submip')} | {rper('bgcopy')} | {rper('restart')} | {hc / hs if hs else 0:.1f} |")


if __name__ == "__main__":
    main()
