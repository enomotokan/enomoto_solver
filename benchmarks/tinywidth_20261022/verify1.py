"""53d41e1 の防御を外して (ENOMOTO_MIP_COEF_TIGHTEN_TINY=1)、固定の処理だけで正しい値になるかを見る。
(1) 書き出した uccase12 の再スタート後の問題を EQUIV2 で直接解く (1 スレッド・60 秒)、(2) fuzz_tiny の seed 41。"""
import json, subprocess, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "psched_ioeq_20261022"))
import run as R
here = Path(__file__).resolve().parent
T = {"ENOMOTO_MIP_COEF_TIGHTEN_TINY": "1"}; NT = {"ENOMOTO_MIP_NO_TINY_WIDTH_FIX": "1"}
for name, env in [("seed41_fix", T), ("seed41_old", {**T, **NT}), ("seed41_default", {})]:
    p = subprocess.run([sys.executable, str(here.parents[0] / "equiv2_bug_20261022/fuzz_tiny.py"), "1", "41"], capture_output=True, text=True, env={**__import__("os").environ, **env})
    print(name, p.stdout.strip().replace("\n", " | "), flush=True)
for name, env in [("e2e_fix", T), ("e2e_old", {**T, **NT})]:
    r, err = R.run(here / "e2e/r1.mps", 60.0, 1, {"ENOMOTO_MIP_PROBE_EQUIV2": "1", "ENOMOTO_MIP_NO_LP_PROBE": "1", **env}, log=True)
    (here / f"e2e/{name}.log").write_text(err)
    (here / f"e2e/{name}.json").write_text(json.dumps(r))
    print(name, json.dumps({k: r.get(k) for k in ("status", "objective", "bound", "root_bound")}),
          [l for l in err.splitlines() if "width within" in l or "equivalence pass" in l or "(skipped" in l][:6], flush=True)
