"""待ち行列 3 (続きから回せる): 58 問 (4 スレッド・60 秒、種 1・2) の def と a -> bench58.jsonl、
その後 a の効く問題の 600 秒 (種 1・2) -> long.jsonl"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
sys.path.insert(0, str(D.parents[0] / "cbcimpl_20261029"))
from queue1 import run  # noqa: E402
from probs import PROBS  # noqa: E402
for rep in (1, 2):
    for p in PROBS:
        for cfg in (["def", "a"] if rep == 1 else ["a", "def"]):
            run(p, cfg, 60, "bench58.jsonl", rep=rep, root=False)
for rep in (1, 2):
    for p, cfgs in [("thor50dday", ["def", "a"]), ("lectsched-5-obj", ["def", "a"]), ("sp98ar", ["def", "a"]), ("physiciansched6-2", ["def", "a", "mk", "mka"]), ("tbfp-network", ["def", "a"])]:
        for cfg in cfgs:
            run(p, cfg, 600, "long.jsonl", rep=rep, root=False)
