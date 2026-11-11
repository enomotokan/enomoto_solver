"""IPMRC の分担 K を回す (analysis/ipmrc_20261111.md)。shards.json の K 番目の (問題, 設定, 種) を順に bench.py で 1 回ずつ
(既定 600 秒・4 スレッド、shards.json の "tl") 解き、res_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ。
前もって `python -m venv .venv && .venv/bin/pip install maturin highspy && . .venv/bin/activate && maturin develop --release`。
使い方: .venv/bin/python analysis/ipmrc_20261111/run_share.py K"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
k = int(sys.argv[1])
sh = json.loads((D / "shards.json").read_text())
mine = sh["shards"][k]
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
    subprocess.run([sys.executable, str(D / "bench.py"), str(OUT), str(sh["tl"]), p, c, str(s)], check=False)
