"""bench.py・lpbench.py の設定 (LUCLOCK)。"""
HEAVY = ["satellites2-60-fs", "neos-3216931-puriri", "neos-933966", "mzzv11", "neos-4722843-widden", "physiciansched6-2", "mzzv42z",
         "seymour1", "cvs16r128-89", "neos-827175", "unitcal_7"]
R3 = {"ENOMOTO_MIP_ROOTLP3": "3"}
CFG = {
    "def": {},
    "b": {"ENOMOTO_SIMPLEX_LUCLOCK": "b"},
    "g": {"ENOMOTO_SIMPLEX_LUCLOCK": "g"},
    "r3": dict(R3),
    "r3b": {**R3, "ENOMOTO_SIMPLEX_LUCLOCK": "b"},
    "r3g": {**R3, "ENOMOTO_SIMPLEX_LUCLOCK": "g"},
    "r3e50": {**R3, "ENOMOTO_T_RACE_BOUND_EVERY": "50"},
    "r3e200": {**R3, "ENOMOTO_T_RACE_BOUND_EVERY": "200"},
    "r3be50": {**R3, "ENOMOTO_SIMPLEX_LUCLOCK": "b", "ENOMOTO_T_RACE_BOUND_EVERY": "50"},
    "f15": {"ENOMOTO_SYNTH_CLOCK_FACTOR": "15"},
    "f17": {"ENOMOTO_SYNTH_CLOCK_FACTOR": "17"},
    "bf15": {"ENOMOTO_SIMPLEX_LUCLOCK": "b", "ENOMOTO_SYNTH_CLOCK_FACTOR": "15"},
    "bf17": {"ENOMOTO_SIMPLEX_LUCLOCK": "b", "ENOMOTO_SYNTH_CLOCK_FACTOR": "17"},
    "g09": {"ENOMOTO_SIMPLEX_LUCLOCK": "g", "ENOMOTO_T_LUCLOCK_G_FACTOR": "0.9375"},
    "g11": {"ENOMOTO_SIMPLEX_LUCLOCK": "g", "ENOMOTO_T_LUCLOCK_G_FACTOR": "1.0625"},
    "g4": {"ENOMOTO_SIMPLEX_LUCLOCK": "g", "ENOMOTO_T_LUCLOCK_G_FACTOR": "0.25"},
    "g025": {"ENOMOTO_SIMPLEX_LUCLOCK": "g", "ENOMOTO_T_LUCLOCK_G_FACTOR": "4"},
}
