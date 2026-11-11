#!/bin/bash
# CUTMIX の計測の順番 (凍結した b2)。固定費用型の問題 → 58 問 → 47 問の型の診断 (短い時間)
export CM_PKG=/tmp/claude-0/-home-user-enomoto-solver/9d19be4d-6279-5c6c-9f4e-5a48cf5d1d10/scratchpad/cm/b2
export CM_EXTRA_CFG='cmh=ENOMOTO_MIP_CUTMIX:h;cmfh=ENOMOTO_MIP_CUTMIX:fh'
PY=../../.venv/bin/python
FC=h80x6320d,p200x1188c,sp150x300d,mc11,rout,gr4x6,beavma,ran14x18-disj-8,drayage-25-23
$PY bench.py h58.jsonl 60 $FC def,cmh,cmfh 1,2
$PY bench.py h58.jsonl 60 p58 def,cmh,cmfh 1,2
$PY bench.py t47.jsonl 20 m47 def 1
echo QUEUE_DONE
