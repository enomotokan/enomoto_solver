"""最適解を ENOMOTO_MIP_START_FILE で与えて解き、根で止まるか・返す値が正しいかを見る (根で暫定解 = 下界の状況を作る)。
使い方: python startfile.py OUT.jsonl TL PROB... (環境変数はそのまま子に渡す。LABEL=... で記録の名前)"""
import json, os, re, subprocess, sys, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; CACHE = ROOT / ".miplib_cache"; SOLS = Path(__file__).resolve().parent / "sols"

def opt_solution(name):
    f = SOLS / f"{name}.sol"
    if f.exists():
        return f
    import highspy
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("time_limit", 300.0)
    h.readModel(str(CACHE / f"{name}.mps")); h.run()
    if h.getModelStatus() != highspy.HighsModelStatus.kOptimal:
        return None
    SOLS.mkdir(exist_ok=True)
    f.write_text("\n".join(repr(v) for v in h.getSolution().col_value) + "\n")
    (SOLS / f"{name}.obj").write_text(repr(h.getInfo().objective_function_value))
    return f

