"""候補ごとに MIPLIB 40 問 (threads=4、60 秒) を回す。回ごとに全候補を一巡し、結果は ev_<名前>_<回>.jsonl。
途中で止めても同じコマンドで続きから再開できる (drive.py が解き終えた問題を飛ばす)。"""
import os
import subprocess
import sys

SP = "/tmp/claude-0/-home-user-enomoto-solver/89e57c1f-baeb-54a0-97e3-7cc4061b50b5/scratchpad"
D = f"{SP}/treepar"
NEW, OLD = f"{SP}/venv/bin/python", f"{SP}/venv_before/bin/python"
CANDS = [
    ("kb", NEW, {}),
    ("kseq", NEW, {"ENOMOTO_MIP_PATH_SEQ": "1"}),
    ("kdive", NEW, {"ENOMOTO_MIP_PAR_DIVES": "1"}),
]
ROUNDS = [(1, None), (2, None)]
for rnd, only in ROUNDS:
    for name, py, env in CANDS:
        if only is not None and name not in only:
            continue
        out = f"{D}/kv_{name}_{rnd}.jsonl"
        e = dict(os.environ, **env)
        print(f"round {rnd} {name}", flush=True)
        subprocess.run([py, f"{D}/drive.py", "4", "60", out], env=e, stdout=open(f"{D}/ev.log", "a"), stderr=subprocess.STDOUT)
print("all done", flush=True)
