"""LUCLOCK の 58 問 (60 秒・4 スレッド) の分担: 分担表 shards.json の K 番目の (問題, 種) を、設定 def・b・g で 1 回ずつ解き、
res_sK.jsonl に足す (bench.py を問題ごとに呼ぶ。行の形は bench.py と同じ。途中から再開できる)。ソルバーは同時に 1 つ。
前もって固定のコミットで `maturin develop --release` しておくこと (.venv の python で走らせる)。
使い方: python analysis/luclock_20261111/run_shard.py K"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
k = int(sys.argv[1])
plan = json.loads((D / "shards.json").read_text())
for p, s in plan["shards"][k]:
    subprocess.run([sys.executable, str(D / "bench.py"), f"res_s{k}.jsonl", str(plan["tl"]), p, ",".join(plan["cfgs"]), str(s)], check=False)
