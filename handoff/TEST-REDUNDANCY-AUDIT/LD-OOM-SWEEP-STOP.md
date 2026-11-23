# LD-OOM-SWEEP-STOP: proposed by TEST-REDUNDANCY-AUDIT (M6)

Id: LD-OOM-SWEEP-STOP
Kind: code
Priority: 40
Size: S
Model: opus
Effort: high
Premise: dlopen_new_objects_out_of_memory_is_clean (58.5 s) and start_objects_out_of_memory_is_clean (27.9 s) sweep every limit to 48 MiB; dlopen_process_objects' sweep stops once two runs in a row complete (a larger limit only adds room no growth path lacks).

## Ruling

Reduces a repeat count: needs the owner's ruling with the measured saving.

## The change

Print the first completed limit of each sweep and measure the share of runs above the second completed one; if it is large, apply the same stop rule, with its reason in the test.

## Proof

Measured share and seconds; plants: a failure planted at the first growth path (below the stop) is still caught.

## Context

Read plan/reviews/TEST-REDUNDANCY-AUDIT.md (item M6) first: it has the evidence, the timing sources and the risk.
Measure before/after from the next CI gates' nextest lines (the report's T1 method), not from an estimate.
