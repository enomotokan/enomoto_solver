cd "$(dirname "$0")"
for s in 2 3 4 5 6 7 8 9 10; do ./run1.sh t4_s$s 4 $s; done
for s in 1 2 3 4 5 6 7 8; do ./run1.sh ipm_t4_s$s 4 $s ENOMOTO_MIP_MC11FIX1_RACE=i; done
for s in 1 2 3 4; do MPS=neos859080_obj2.mps ./run1.sh obj2_t4_s$s 4 $s; done
