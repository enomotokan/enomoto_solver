"""`run.py --variants` の結果 (1 つの JSON Lines に案が混在) を案ごとのファイル `<出力の接頭辞><案>.jsonl` に分ける。

    python scripts/crossover_bench/split_methods.py benchmarks/crossover/ab23/all.jsonl benchmarks/crossover/ab23/
"""
import json
import sys
from pathlib import Path

src, prefix = sys.argv[1], sys.argv[2]
out = {}
for line in open(src):
    r = json.loads(line)
    out.setdefault(r["method"], []).append(line)
for m, lines in out.items():
    Path(prefix + m + ".jsonl").write_text("".join(lines))
    print(prefix + m + ".jsonl", len(lines))
