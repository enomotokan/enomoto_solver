"""待ち行列 2 (続きから回せる): 上限に当たった問題 (flagged.json) の根 (90 秒、ノード上限 1) を def・a・n・c・ac で 1 回ずつ -> rootcmp.jsonl。
physiciansched6-2 は MODKVB と合わせた mk・mka・mkn・mkc も (60 秒の根)"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from queue1 import run  # noqa: E402
for p in ["physiciansched6-2"]:
    for cfg in ["mk", "mka", "mkn", "mkc"]:
        run(p, cfg, 60, "rootcmp_mk.jsonl")
for cfg in ["def", "a", "n", "c", "ac"]:
    for p in json.load(open(D / "flagged.json")):
        run(p, cfg, 90, "rootcmp.jsonl")
