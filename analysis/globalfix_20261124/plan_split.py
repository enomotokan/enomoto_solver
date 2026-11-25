"""切り分けの分担表 split.json を作る: python plan_split.py。
=abc の 4 スレッドで害の出た問題 (noswot 1.34・10teams 1.32) と、1.05 倍を超えた問題 (pk1・neos-911970) について、
=a・=b・=c 単独を種 1〜5、既定と =abc を種 4・5 で足す (種 1〜3 は res_s3.jsonl)。1 スレッドで 1.05 倍を超えた noswot・pk1 も同じ形で足す。
4 スレッドの回と 1 スレッドの回を 1 本の分担に並べ、ほかと並べずに順に回す (結果は res_x0.jsonl)。"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
sh = []
for th, probs in (("t4", ["noswot", "10teams", "pk1", "neos-911970"]), ("t1", ["noswot", "pk1"])):
    for p in probs:
        sh += [(p, f"{c}_{th}", s, 60) for s in (1, 2, 3, 4, 5) for c in ("a", "b", "c")]
        sh += [(p, f"{c}_{th}", s, 60) for s in (4, 5) for c in ("def", "abc")]
(D / "split.json").write_text(json.dumps({"shards": [sh]}))
print(len(sh))
