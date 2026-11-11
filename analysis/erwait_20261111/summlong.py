"""ERWAIT: reslong.jsonl と logs/ から、600 秒の回ごとの結果と「早めの再スタートを決めてから再スタートまで」の秒 (ERWAIT_DIAG の最初の
restart の行) を表にする。使い方: python analysis/erwait_20261111/summlong.py"""
import gzip, json, re
from pathlib import Path
D = Path(__file__).resolve().parent
print("| 問題 | 種 | 設定 | 状態 | 秒 | 目的値 | 根の下界 | ノード | 決めてから再スタートまで (秒、1 回目) | 最大 RSS (MB) |")
print("|---|---|---|---|---|---|---|---|---|---|")
rows = [json.loads(l) for l in (D / "reslong.jsonl").read_text().splitlines() if l.strip()]
for r in sorted(rows, key=lambda r: (r["problem"], r["seed"], r["cfg"])):
    res = r["result"]
    log = D / "logs" / f"{r['problem']}_{r['build']}_{r['cfg']}_s{r['seed']}_{r['tl']}.log.gz"
    w = "-"
    if log.exists():
        m = re.search(r"ERWAIT_DIAG: restart \+([0-9.]+)s", gzip.decompress(log.read_bytes()).decode(errors="replace"))
        if m:
            w = m.group(1)
    tm = res.get("time") or 0
    print(f"| {r['problem']} | {r['seed']} | {r['cfg']}{'' if r['build'] == 'ew' else ' (' + r['build'] + ')'} | {res.get('status')} | {tm:.1f} | {res.get('objective')} | {res.get('root_bound')} | {res.get('nodes')} | {w} | {r['max_rss_mb']} |")
