"""ROOTLP3 の分担 K を回す (analysis/rootlp3_20261111.md §4)。shards.json の K 番目の (問題, 設定, 種) を順に bench.py で 1 回ずつ
(60 秒・4 スレッド) 解き、res_sK.jsonl に足す。途中から再開できる (済んだ (問題, 設定, 種) は飛ばす)。ソルバーは同時に 1 つ。
使い方: python analysis/rootlp3_20261111/run_share.py K"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
k = int(sys.argv[1])
mine = json.loads((D / "shards.json").read_text())["shards"][k]
OUT = D / f"res_s{k}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["cfg"], r["rep"]))
for p, c, s in mine:
    if (p, c, s) in done:
        continue
    subprocess.run([sys.executable, str(D / "bench.py"), str(OUT), "60", p, c, str(s), "2"], check=False)
