# STRPTIME-SIGNED-YEAR: strptime's number conversions take no sign (GitHub issue 18)

State: branch task/strptime-signed-year. Code change in `crates/core/src/time/parse.rs`: `number()` and `%s` no longer
read a leading `+` or `-`; `%z` keeps its sign. Tests: `s23_strptime_engine` (87 + new rows, each fails on next),
`time_gaps_probe` and `timefmt_products` (the recorded "kept" sign differences removed), unit tests in parse.rs.
Plants: `handoff/STRPTIME-SIGNED-YEAR-plants.txt`.

## Decision in force

D-23.7 had kept a difference: libniji accepted a sign on `%Y %C %y %s` (POSIX). The brief and issue 18 (systemd's
os-release test needs `strptime("-1-1-1", "%F")` to fail) ask for the reference's behaviour, so that difference ends:
its row in `docs/differences-from-glibc.md` is removed, the row in `spec/behaviour/time-gaps.md` now says Equal,
and the KNOWN entries in `time_gaps_probe.rs` and `timefmt_products.rs` are removed. No compat flag was offered
(the old row had none; a new flag for a sign the reference rejects would restore a known mismatch).

## Probe table (black box, C locale; 29 inputs x 31 formats, both libraries; status only, "NULL" or consumed length)

Reference: every input beginning with `-` or `+` (also after leading spaces) is NULL for `%Y %F %G %C %y %s %Ey %EY %Oy`
and for `%C%y`, `%m %d %e %H %I %M %S %j %u %w %U %V %g` and the composites `%D %T %R`, `%Y-%m-%d`.
`%z` takes `+2024`, `-2024`, `+12`, `-12`, `+12345`, `-12345` (a sign is part of the format).
Unsigned: `%F 2024-13-01` NULL, `%F 2024-02-30` parsed (mon=1 mday=30 wday=5 yday=60).

| input | formats | reference | libniji before | libniji after |
|---|---|---|---|---|
| `-1-1-1`, `+1-1-1` | `%F` | NULL | consumed 6, year -1901 / -1899 | NULL |
| `-1`, `+1`, ` -1`, ` +1`, `-0`, `+2024`, `-2024`, ... | `%Y %G %C %s %EY` | NULL | consumed, signed value | NULL |
| same | `%y %Ey %Oy` | NULL | `+n` consumed, `-n` NULL | NULL |
| `-1-5`, `20+5` | `%C%y` | NULL | consumed | NULL |
| `-1`, `+1`, `-1:00`, ... | `%m %d %H` | NULL | NULL | NULL |
| `+2024`, `-12`, ... | `%z` | accepted | accepted | accepted |
| `2024-13-01` | `%F` | NULL | NULL | NULL |
| `2024-02-30` | `%F` | parsed | parsed | parsed |

Of 899 probe rows, 168 differed in NULL-or-consumed-length before; after the change only two differ (the unsigned
`%Ey` rows below). The remaining field-level differences (`tm_wday`/`tm_yday` when the input gave no date) are the
documented D-23.7 difference, which this probe provokes by pre-setting `tm_mday`.

## Open

INFORMATIONAL (not changed, outside the brief): `%Ey` on `2024-13-01` and `2024-02-30` is NULL in the reference
and consumes 2 digits in libniji (`%Ey` on a 2-digit input was not probed in that table). Follow-up story if wanted.

## Next step

None for this story; gate and plants labels are in the report.
