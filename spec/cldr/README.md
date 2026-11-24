# Unicode CLDR data for the generated locale sources (D-31.27)

`49/` holds what `cargo xtask locale-sources` (`xtask/src/locales.rs`) reads, extracted from the pinned
Unicode CLDR release by `tools/cldr/cldr_extract.py` (the URL and SHA-256 are in that script and in the
first line of each `.tsv`). The build never fetches anything; to move the pin, change `PIN_*` in the
script, run `python3 -I tools/cldr/cldr_extract.py` and review the diff of `49/` (renamed to the new
version) like any other change.

* `49/locales.tsv`: per locale name, the CLDR values resolved through CLDR's inheritance.
* `49/territories.tsv`: per territory, CLDR's supplemental data (currency, paper, measurement, week).
* `49/allkeys_CLDR.txt`: CLDR's root collation, unchanged.
* `49/LICENSE`: the Unicode License v3, under which all of the above is distributed; the `locales`
  package's copyright file carries the notice.

`locale-names.tsv` is the list of locale names libniji's `locales` package covers (the names, not any
data, of the reference system's list); names CLDR does not cover are listed there and not generated.
