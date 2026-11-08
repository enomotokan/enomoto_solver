#!/bin/bash
cd "$(dirname "$0")"
export CFGS="def nonodecut norestart nosb"
./abl.sh csched008 300
./abl.sh unitcal_7 300
./abl.sh tr12-30 450
echo ALLDONE
