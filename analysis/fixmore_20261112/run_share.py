"""FIXMORE の分担 K を回す (analysis/fixmore_20261112.md)。shards.json の K 番目の (設定, 問題, 時間, 種) を順に run.py で 1 回ずつ
(4 スレッド) 解き、res_sK.jsonl に足す。途中から再開できる (済んだものは飛ばす)。ソルバーは同時に 1 つ。
使い方 (リポジトリの根で): sh analysis/fixmore_20261112/build.sh && python analysis/fixmore_20261112/run_share.py K"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
k = int(sys.argv[1])
# `--file shards45.json` で別の分担の表 (出力は res45_sK.jsonl)
fname = sys.argv[sys.argv.index("--file") + 1] if "--file" in sys.argv else "shards.json"
tag = "" if fname == "shards.json" else fname.replace("shards", "").replace(".json", "")
mine = json.loads((D / fname).read_text())["shards"][k]
# 計測中は追跡しない live ファイルに書き、コミットのときに res_sK.jsonl へ写す (safe_commit.sh。git の取り込みが追記とぶつからないように)
out = f"res{tag}_s{k}.live.jsonl" if "--live" in sys.argv else f"res{tag}_s{k}.jsonl"
done = set()
for o in {out, f"res{tag}_s{k}.jsonl"}:
    if (D / o).exists():
        for l in (D / o).read_text().splitlines():
            if l.strip():
                r = json.loads(l)
                done.add((r["cfg"], r["problem"], int(r["tl"]), r["rep"]))
for c, p, tl, s in mine:
    if (c, p, int(tl), s) in done:
        continue
    subprocess.run([sys.executable, str(D / "run.py"), c, p, str(tl), str(s), out], check=False)
print("SHARE DONE", k, flush=True)
