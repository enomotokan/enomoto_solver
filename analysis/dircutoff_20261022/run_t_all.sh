#!/bin/bash
D=$(cd $(dirname $0) && pwd)
$D/step_t.sh 2>&1 | sed "s|$D/sols/||g" > $D/step_t.txt
$D/rootloop_t.sh > $D/rootloop_t.txt 2>&1
$D/batch_t.sh > $D/batch_t.txt 2>&1
