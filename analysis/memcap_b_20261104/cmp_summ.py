"""runs.jsonl の比べる回 (run_cmp.sh) を表にする: 結果・ノード・最大 RSS・RSS の推移 (120/240/360/480/600 秒)・終わりの持ち物 (2026-11-04)。"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
R = re.compile(r"MEMB_DIAG: tid (\d+) .*?queue n (\d+) slots \d+ changes ([\d.]+) basis ([\d.]+) dse ([\d.]+) .*?\| pool n (\d+) slots \d+ changes ([\d.]+) basis ([\d.]+) dse ([\d.]+) .*?row_log add \d+ del \d+ ([\d.]+) MB")
recs = [json.loads(l) for l in open(D / "runs.jsonl")]
for prob in ("uccase12", "CMS750_4"):
    print(f"\n### {prob}\n")
    print("| 回 | 環境変数 | 結果 | 目的値 | 下界 | ノード | 最大 RSS [MB] | RSS 120/240/360/480/600 s [GB] | 終わりの持ち物: 各スレッド 基底/DSE・共有 基底/DSE [MB] (共有のノード数) |")
    print("|---|---|---|---:|---:|---:|---:|---|---|")
    for r in recs:
        if r["problem"] != prob or r["time_limit"] != 600 or r["tag"].startswith(("diag", "ht")):
            continue
        env = " ".join(f"{k}={v}" for k, v in r["env"].items() if k != "ENOMOTO_MIP_MEMB_DIAG") or "(既定)"
        ser = r["series"]
        at = lambda t: min(ser, key=lambda x: abs(x[0] - t))[1] / 1024
        last = {}
        for l in r["diag"]:
            m = R.match(l)
            if m: last[int(m.group(1))] = m.groups()
        qb = sum(float(g[3]) for g in last.values()); qd = sum(float(g[4]) for g in last.values())
        g0 = last.get(0)
        pb, pd, pn = (float(g0[7]), float(g0[8]), int(g0[5])) if g0 else (0, 0, 0)
        res = r["result"]
        print(f"| {r['tag']} | {env} | {res.get('status')} | {res.get('objective'):.6f} | {res.get('bound'):.6f} | {res.get('nodes')} | {r['peak_rss_mb_sampled']} | "
              + " / ".join(f"{at(t):.2f}" for t in (120, 240, 360, 480, 600)) + f" | {qb:.0f}/{qd:.0f}・{pb:.0f}/{pd:.0f} ({pn}) |")
