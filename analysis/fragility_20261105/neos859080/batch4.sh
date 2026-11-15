cd "$(dirname "$0")"
for s in 1 2 3 4 5 6 7 8 9 10; do ./run1.sh rob2_t4_s$s 4 $s ENOMOTO_MIP_ROBUST_ROOTIPM=1; done
for s in 1 2 3; do ./run1.sh rob2_t1_s$s 1 $s ENOMOTO_MIP_ROBUST_ROOTIPM=1; done
