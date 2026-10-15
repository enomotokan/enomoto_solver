"""候補ごとに MIPLIB 40 問 (threads=4、60 秒) を回す。回ごとに全候補を一巡し、結果は ev_<名前>_<回>.jsonl。
途中で止めても同じコマンドで続きから再開できる (drive.py が解き終えた問題を飛ばす)。"""
import os
import subprocess
import sys

SP = "/tmp/claude-0/-home-user-enomoto-solver/89e57c1f-baeb-54a0-97e3-7cc4061b50b5/scratchpad"
D = f"{SP}/treepar"
NEW, OLD = f"{SP}/venv/bin/python", f"{SP}/venv_before/bin/python"
OFF = {"ENOMOTO_MIP_PROOF_NO_AGING": "1", "ENOMOTO_T_MIP_PROOF_POOL_PER_COL": "0", "ENOMOTO_MIP_TREE_SHARE_LITS_AT_ONCE": "1",
       "ENOMOTO_MIP_TREE_NO_BALANCE": "1", "ENOMOTO_MIP_TREE_NO_PASS_BASIS": "1", "ENOMOTO_T_MIP_TREE_START_NODES": "2",
       "ENOMOTO_T_MIP_TREE_HEUR_DIV_INC": "4", "ENOMOTO_T_MIP_TREE_SHARE_PROOF_USES": "1"}
CANDS = [
    ("fin", NEW, {}),
    ("finoff", NEW, OFF),
]
ROUNDS = [(1, None), (2, None)]
for rnd, only in ROUNDS:
    for name, py, env in CANDS:
        if only is not None and name not in only:
            continue
        out = f"{D}/fv_{name}_{rnd}.jsonl"
        e = dict(os.environ, **env)
        print(f"round {rnd} {name}", flush=True)
        subprocess.run([py, f"{D}/drive.py", "4", "60", out], env=e, stdout=open(f"{D}/fv.log", "a"), stderr=subprocess.STDOUT)
print("all done", flush=True)
