#!/bin/bash
cd $(dirname $0)
export HIGHS_BIN=${HIGHS_BIN:-/tmp/claude-0/-home-user-enomoto-solver/613fdde1-2219-543d-8467-14e982298b93/scratchpad/HiGHS/build/bin/highs}
for P in roll3000 mzzv42z 30n20b8 neos-911970 physiciansched6-2; do ./run_highs.sh $P ${P}_h 60; done
