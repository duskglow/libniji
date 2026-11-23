Window: runs started on or after 2026-10-24, as of 2026-11-23T17:30:00Z. Chains: 689; cost $4665; realized gain items 516; overall $9 per realized gain item. Cost estimated from tokens: $953. Hubs not used as chain edges: 21.

By size (stories in chain):

| stories | chains | cost $ | $/chain | landed % | realized gain/chain | $/realized gain |
|---|---|---|---|---|---|---|
| 1-1 | 362 | 1108 | 3.1 | 83 | 0.67 | 5 |
| 2-2 | 130 | 671 | 5.2 | 89 | 0.59 | 9 |
| 3-3 | 68 | 724 | 10.6 | 90 | 1.25 | 9 |
| 4-5 | 74 | 846 | 11.4 | 95 | 0.91 | 13 |
| 6-99 | 55 | 1317 | 23.9 | 95 | 0.82 | 29 |

Red causes (all chains): own 537, incomplete 254, ci-conflict 155, ci-shared 105, coordinator 74, env 48

Runs in chains of 1-2 stories: 625; hand-back result mentions merging next: 13 (2%), $18.

Runs in chains of 3-5 stories: 528; hand-back result mentions merging next: 54 (10%), $101.

Runs in chains of 6+ stories: 604; hand-back result mentions merging next: 67 (11%), $114.

Most frequent failing tests named in reds since 2026-11-16: release start 14; niji::deb_manifest tables_are_sorted_well_formed_and_name_built_products 11; niji::hot_text_products init_dynamic_empty_main_stays_within_its_instruction_bound 11; niji::heap_meta_products the_heap_keeps_address_space_near_its_use 7; niji::aio_engine environment_selects_threads 3; niji::aio_engine kernel_refusal_selects_threads 3

Cost per run (recorded cost only): 11-10..11-16: 550 runs, $3.03/run; 11-17..11-23: 453 runs, $4.51/run

| rank | chain | stories | continuations | depth | pending | runs | legs | cost_usd | cost_estimated_usd | agent_hours | reds | red_own | red_incomplete | red_env | red_ci_shared | red_ci_conflict | red_coordinator | landed_stories | impact_recorded | gain_items | gain_realized | cost_rows | usd_per_realized_gain | first_run | last_run |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | LAYOUT-TEXT-PIN | 31 | 30 | 15 | 2 | 31 | 31 | 88 | 0 | 30.6 | 30 | 13 | 11 | 0 | 2 | 2 | 2 | 5 | 2 | 2 | 0 | 1 | inf | 2026-11-10 | 2026-11-14 |
| 2 | TB-FOREIGN-WORDS-LOW | 27 | 26 | 14 | 0 | 27 | 27 | 80 | 0 | 28.2 | 27 | 15 | 5 | 2 | 0 | 3 | 2 | 2 | 15 | 20 | 0 | 20 | inf | 2026-11-10 | 2026-11-14 |
| 3 | TIMING-COV-MEMCMP-NONAVX2 | 27 | 26 | 4 | 0 | 27 | 0 | 57 | 57 | 24.6 | 15 | 3 | 7 | 0 | 2 | 3 | 0 | 14 | 0 | 0 | 0 | 0 | n/r | 2026-11-05 | 2026-11-06 |
| 4 | LIBM-F64-TRIG-QUICK | 18 | 17 | 3 | 0 | 18 | 1 | 50 | 48 | 21.4 | 21 | 8 | 5 | 0 | 3 | 5 | 0 | 8 | 0 | 0 | 0 | 0 | n/r | 2026-11-07 | 2026-11-10 |
| 5 | LIBM-SPECIAL-FAST-TIER | 19 | 18 | 6 | 0 | 19 | 0 | 44 | 44 | 19.9 | 20 | 4 | 4 | 1 | 0 | 11 | 0 | 6 | 0 | 0 | 0 | 0 | n/r | 2026-11-05 | 2026-11-06 |
| 6 | DL-SEARCH-CACHE | 14 | 13 | 4 | 0 | 14 | 14 | 40 | 0 | 15.4 | 9 | 6 | 1 | 0 | 0 | 2 | 0 | 5 | 0 | 0 | 0 | 0 | n/r | 2026-11-10 | 2026-11-10 |
| 7 | STARTUP-FAULTS-RESTORE | 15 | 14 | 5 | 0 | 15 | 12 | 39 | 8 | 15.7 | 10 | 10 | 0 | 0 | 0 | 0 | 0 | 7 | 0 | 0 | 0 | 0 | n/r | 2026-11-07 | 2026-11-10 |
| 8 | CLASSED-SLOT-INFRA | 13 | 12 | 7 | 0 | 13 | 0 | 34 | 34 | 12.6 | 9 | 5 | 4 | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | n/r | 2026-11-05 | 2026-11-06 |
| 9 | PRINTF-NO-HEAP | 3 | 2 | 3 | 0 | 3 | 9 | 34 | 0 | 9.4 | 3 | 2 | 1 | 0 | 0 | 0 | 0 | 0 | 3 | 15 | 0 | 5 | inf | 2026-11-23 | 2026-11-23 |
| 10 | INTERNAL-ALLOC-ROUTING | 3 | 2 | 4 | 1 | 3 | 8 | 29 | 0 | 7.8 | 2 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 3 | 7 | 0 | 5 | inf | 2026-11-22 | 2026-11-22 |
| 11 | SHSTK-OPT-IN | 3 | 2 | 2 | 0 | 3 | 7 | 29 | 0 | 5.7 | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 3 | 4 | 0 | 10 | inf | 2026-11-23 | 2026-11-23 |
| 12 | LOCALE-PROBE-GEN | 4 | 3 | 2 | 0 | 4 | 5 | 29 | 0 | 6.8 | 6 | 1 | 0 | 1 | 4 | 0 | 0 | 0 | 4 | 4 | 0 | 1 | inf | 2026-11-23 | 2026-11-23 |
| 13 | LIBM-F32-ONEPASS-REST | 10 | 9 | 5 | 0 | 10 | 10 | 27 | 0 | 9.6 | 8 | 4 | 2 | 1 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | n/r | 2026-11-10 | 2026-11-10 |
| 14 | HEAP-ADDRESS-SPACE | 10 | 9 | 10 | 0 | 11 | 14 | 50 | 0 | 20.0 | 11 | 6 | 4 | 1 | 0 | 0 | 0 | 1 | 10 | 16 | 1 | 13 | 50 | 2026-11-17 | 2026-11-20 |
| 15 | UTIL | 9 | 8 | 4 | 1 | 9 | 14 | 49 | 0 | 16.3 | 6 | 1 | 2 | 0 | 1 | 2 | 0 | 3 | 9 | 9 | 1 | 1 | 49 | 2026-11-17 | 2026-11-23 |
| 16 | WCTYPE-TRIE-FILE | 13 | 12 | 11 | 0 | 13 | 11 | 40 | 4 | 17.6 | 12 | 10 | 1 | 0 | 0 | 1 | 0 | 2 | 5 | 9 | 1 | 1 | 40 | 2026-11-07 | 2026-11-15 |
| 17 | DL-DLCLOSE | 5 | 4 | 5 | 0 | 5 | 5 | 28 | 0 | 8.0 | 4 | 2 | 1 | 0 | 0 | 1 | 0 | 1 | 5 | 5 | 1 | 1 | 28 | 2026-11-14 | 2026-11-14 |
| 18 | HEADERS-TABLES-COMPLETE | 5 | 4 | 2 | 0 | 5 | 11 | 48 | 0 | 7.9 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 4 | 4 | 5 | 3 | 0 | 16 | 2026-11-19 | 2026-11-23 |
| 19 | DL | 3 | 2 | 1 | 0 | 3 | 7 | 27 | 0 | 8.5 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 2 | 3 | 4 | 2 | 8 | 14 | 2026-11-19 | 2026-11-21 |
| 20 | DL-LIBC-START | 34 | 33 | 10 | 0 | 34 | 34 | 112 | 0 | 34.4 | 30 | 19 | 5 | 0 | 2 | 4 | 0 | 9 | 17 | 16 | 9 | 1 | 12 | 2026-11-10 | 2026-11-13 |
| 21 | DL-CANONICAL-PLT | 7 | 6 | 7 | 0 | 7 | 10 | 37 | 0 | 13.1 | 6 | 1 | 4 | 0 | 0 | 0 | 1 | 1 | 7 | 9 | 3 | 15 | 12 | 2026-11-17 | 2026-11-19 |
| 22 | HEAP-FIRST-USE | 11 | 10 | 8 | 0 | 12 | 12 | 32 | 0 | 13.1 | 9 | 5 | 3 | 1 | 0 | 0 | 0 | 3 | 5 | 6 | 3 | 3 | 11 | 2026-11-10 | 2026-11-14 |
| 23 | DYN-START-PARITY | 5 | 4 | 3 | 0 | 5 | 7 | 28 | 0 | 12.1 | 3 | 2 | 0 | 0 | 0 | 1 | 0 | 2 | 5 | 13 | 3 | 1 | 9 | 2026-11-19 | 2026-11-20 |
| 24 | LOCALE-SLOTS-GROWTH | 7 | 6 | 7 | 0 | 8 | 8 | 28 | 0 | 10.0 | 7 | 3 | 2 | 1 | 0 | 0 | 1 | 1 | 7 | 12 | 4 | 8 | 7 | 2026-11-17 | 2026-11-20 |
| 25 | LIBM-F32-BESSEL-LARGE | 11 | 10 | 7 | 0 | 11 | 11 | 34 | 0 | 11.4 | 8 | 3 | 3 | 0 | 0 | 2 | 0 | 3 | 6 | 11 | 7 | 4 | 5 | 2026-11-12 | 2026-11-14 |
| 26 | DL-PROCESS-OBJECTS | 3 | 2 | 3 | 0 | 3 | 6 | 33 | 0 | 9.3 | 2 | 1 | 1 | 0 | 0 | 0 | 0 | 3 | 3 | 7 | 7 | 1 | 5 | 2026-11-21 | 2026-11-22 |
| 27 | LAYOUT-WINDOW-BINS | 7 | 6 | 7 | 0 | 7 | 7 | 30 | 0 | 11.3 | 4 | 2 | 2 | 0 | 0 | 0 | 0 | 3 | 7 | 18 | 7 | 2 | 4 | 2026-11-14 | 2026-11-15 |
