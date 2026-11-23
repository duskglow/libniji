# LOCALEDEF-TRANSLIT: localedef transliterates strings the target charmap lacks (compile time)

**State.** Branch `task/localedef-translit` (from origin/next 0495c4684), role shiki `~/w/sh1`. Code head 3aac89895
(the commit with this note sits on top; it changes no code). Scope per coordinator note on #7957: step 2 only
(compile-time transliteration on hand-written sources); steps 3-4 (generator emits the shared translit table,
the 251-value test, the Locale sources differences row) are a follow-up stacked on LOCALE-PROBE-GEN-2 (not landed,
not merged here by ruling).

## Done (do not redo)

* Probes (own sources only, ISO-8859-1, reference localedef + `locale -k` under LOCPATH): shiki
  `~/work/localedef-translit/` (`probe.py` w1, `probe2b.py` w2b, `probe3.sh` w3; outputs p1.txt, p2b.txt). Settled
  rule, now spec clause **s31_8_g_5** and record `spec/behaviour/utilities.glibc.txt` `== localedef-translit`:
  own entry first (later duplicate wins), then includes in order (first wins), recursively; first alternative that
  is non-empty and encodes whole; no re-transliteration, no `default_missing`, no built-in table; LC_CTYPE may
  stand after the category, or be `copy`d. A string that fails is written empty, whole, multibyte AND wide form,
  with `<file>:<line>: <CAT>: unknown character in field `<kw>'` per string (also under --quiet), status 1,
  output written without -c.
* Code: `ctype.rs` (`Pick`, `Ctype::transliterate`), `cats.rs` (`Enc`: charmap + LC_CTYPE; `Out::wide`),
  `lib.rs` (LC_CTYPE parsed once first; `Output::unknown`; `unknown_values`), `prog.rs` (the report lines).
* Tests: `crates/locale/tests/prog.rs` `unencodable_strings_take_the_locales_transliteration` (in memory, no
  reference needed); `crates/locale/tests/reference.rs`
  `strings_the_charmap_lacks_are_transliterated_as_the_reference_does` (every item of the nine string categories vs
  the reference). Plants: `handoff/LOCALEDEF-TRANSLIT.plants` (7).
* Jobs: `localedef-translit-gate2` (reference+prog tests, clippy -D warnings, static, `affected --run --base
  origin/next`, direct tier: 94 targets + 147 conformance, not FULL) and `localedef-translit-plants3`, both on
  3aac89895, started 13:5xZ. SUPERSEDED: t1, t2, lint1 (development; lint1 found the wide-form rule), plants1
  (killed, older head), gate1/plants2 (killed: started while the role was BUSY, old tree).

## Open (INFORMATIONAL, outside this story)

* A defined LC_CTYPE with only a translit section: libniji gives no ASCII classes, maps or widths; the reference does.
* Runtime table (`write_translit`): for `<U2034> "";"A<U2035>";"<U0042>"` the reference stores `[B]`; libniji stores
  an empty first alternative (the list reads back empty). One data point; the spec notes a lone `""` entry.
* Defaults of keywords omitted from a DEFINED category differ (LC_TIME `week` 1stweek 7 vs 4, `date_fmt`;
  LC_ADDRESS `country_ab2/ab3/num`). The reference test now defines them.
* Unprobed, left as before: alt_digits' wide list, era strings' report, a failed-LC_CTYPE's partial table.
* 251 values: the list is on LOCALE-PROBE-GEN-2's branch (not readable here); the follow-up measures it.

## Next step

`~/bin/jobwait localedef-translit-gate2 localedef-translit-plants3` on shiki; on green: precheck
`cargo xtask precheck --jobs localedef-translit-gate2,localedef-translit-plants3 --head <sha>` on the synced
final head, update `handoff/reports/LOCALEDEF-TRANSLIT.json` (gate line), push, `ops handback-check`, hand back.
On red: fix, run only the failing test, rerun.
