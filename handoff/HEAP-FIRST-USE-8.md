# HEAP-FIRST-USE-8: fast first-float red fixed (probe order); one-node release localtime needs a ruling

State: branch `task/heap-first-use-8` = HEAP-FIRST-USE-7's tip 8ce62bb4c + 378fbc4ef (p_strlazy without printf:
superseded by 8df77bc62) + 5ff25ec1e (p_true clock call) + 8df77bc62 (p_strlazy after p_fmt, printf back) +
079ea64f0 (plants files) + this note. No merge in this run (next's merge base is still f6cab48d7).
Role shiki:~/w/sh1 (Haswell-EP, ONE NUMA node). CI-class checks in a RAM copy on mei (Skylake-SP, two nodes):
mei:/mnt/zram/claude/HEAP-FIRST-USE-8/head (no role used there). Scripts: ~/work/HEAP-FIRST-USE-8/ on both hosts.
Status: INCOMPLETE (RULING NEEDED below); do not land until it is ruled.

## Decisions in force
D-3.20 (ask the node only on a multi-node policy). D-X.88 / D-X.92 (every build traces itself). Never commit views.

## What this run found and did
1. Step 0 `hfu8-step0-bounds` (inherited head, shiki): clippy + static green. Bounds: release 3 red (vcall3 not
   grouped; localtime 20 > 18; localtime_mktime 20 > 18), fast 1 red (vcall3), fast float 3. Next (f6cab48d7) on
   shiki: 26/26 in both profiles (`hfu8-next-fast`, `hfu8-next-release`).
2. Fast float cause (key and byte diff of the traced lists before `__snprintf_chk`, ~/work/HEAP-FIRST-USE-8/grp.py):
   p_strlazy (traced after p_str, before p_fmt) runs printf("%s %d\n") + fflush, so `render_simple<ToStream>`
   (7375 B fast), the flush wrapper, `printf_text`, `pair_s` (about 8 KiB) became strings-tier keys AHEAD of the
   snprintf group. Fix 8df77bc62: trace p_strlazy right after p_fmt (its printf must stay: without it
   `liella_hot_memchr_rest` left the stream printf path and release strings read 1 > 0, `hfu8-bounds2`).
3. vcall3 cause (one-node hosts only): start-up no longer runs getcpu (D-3.20), so the vDSO lookup and
   `liella_machine_vcall3` were traced by nothing at start-up. Fix 5ff25ec1e: p_true calls time() (start-up tier),
   which keeps them in the start-up head on every build host, as next had them.
4. Final head, CI class (mei RAM copy, `hfu8-mei-bounds2`): fast 26/26 (float 3, was 4), release 26/26; fork 5
   both; true 10 and 11, tool 49 and 50, locale 46 and 46, localtime 17 and 18 (the two profiles; all within
   their bounds, none above next's level on that class per HEAP-FIRST-USE-7's next run).
   One node (shiki, `hfu8-bounds3`): fast 26/26 (float 3; true 11, tool 49, locale 45 vs next 11/50/46);
   release 24/26: localtime 20 > 18 and localtime_mktime 20 > 18 (next 18; true 10, tool 48, locale 45 vs next 10/49/47).

## RULING NEEDED: first localtime_r on a one-node host (release), 20 > 18
Fault map (`hfu8-fmaploc`, ~/work/HEAP-FIRST-USE-8/fmaploc/{head,next}.map on shiki): whole process 43 faults on
both. Two faults MOVE from start-up into localtime_r: on next start-up's getcpu ran the vDSO lookup (vDSO image
page, `liella::time::UNIVERSAL` page); on the head (one node) the first clock call inside localtime_r runs it:
the vDSO image page (`sys::vdso_fn`) and the `liella_machine_vcall2` text window (0x440000). Not jitter (the head
touches start-up code). Options: (a) p_true also calls clock_gettime so vcall2 joins the start-up head (should take
1 fault back; measure both host classes); (b) for the vDSO page, either look the vDSO up at start-up again (cost:
the lookup's instructions at every start-up, which D-3.20 just removed on one node) or rule that the first clock
call pays the lazy lookup's page (whole-process count unchanged). Recommend (a), then (b) lazy with the bound
counted per node count by the owner's ruling; no bound was changed here.

## Next step
After the ruling: (a) in p_true.c (clock_gettime through a volatile pointer), bounds on both host classes
(bounds.sh on the one-node role, mei_bounds.sh on the CI class), the direct gate, plants (both files).

## Not to redo
-7's fork fix (b032b6f42; fork 5 on both classes). The float diagnosis (step 2). Plants files:
handoff/HEAP-FIRST-USE-8-plants.txt (one node), handoff/HEAP-FIRST-USE-8-plants-ci.txt (CI class).
Plan rows: D-3.20, D-X.88, D-X.92, #4347.
