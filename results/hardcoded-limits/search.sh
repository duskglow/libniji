#!/bin/bash
# AUDIT-HARDCODED-LIMITS: the five search routes of plan/hardcoded-limits.md, rerunnable from the
# repository root: `bash results/hardcoded-limits/search.sh [OUTDIR]` writes one file per route
# (default OUTDIR: a new temporary directory) and prints the hit counts. The product files are the
# shipped crates' src/ trees (spectest, mathtools and test/bench/example directories left out; math
# kept out of routes 1-5 because its constants are numeric, not capacities) and csys/.
set -u
OUT=${1:-$(mktemp -d)}
mkdir -p "$OUT"
find crates -path '*/src/*' -name '*.rs' | grep -v -E '/(tests?|benches|examples)/' \
  | grep -v -E '^crates/(spectest|mathtools)/' | sort > "$OUT/rsfiles.txt"
find csys -type f \( -name '*.c' -o -name '*.h' -o -name '*.S' \) | grep -v -E '/tests?/' | sort > "$OUT/cfiles.txt"
grep -v '^crates/math/' "$OUT/rsfiles.txt" > "$OUT/rs-nomath.txt"

# Route 1: constants and statics named like a capacity.
xargs grep -n -E '^\s*(pub(\([a-z]+\))?\s+)?(const|static)\s+[A-Z0-9_]*(_MAX|_LIMIT|_CAP|_CAPACITY|_SIZE|_DEPTH|_SLOTS|_ENTRIES|_COUNT|_LEN)\b|^\s*(pub(\([a-z]+\))?\s+)?(const|static)\s+(MAX_|LIMIT_|CAP_)[A-Z0-9_]*\b' \
  < "$OUT/rs-nomath.txt" > "$OUT/route1-named.txt"
xargs grep -n -E '#\s*define\s+[A-Z0-9_]*(_MAX|_LIMIT|_CAP|_SIZE|_DEPTH|_SLOTS|_ENTRIES)\b|#\s*define\s+(MAX_|LIMIT_)' \
  < "$OUT/cfiles.txt" >> "$OUT/route1-named.txt"

# Route 2: fixed arrays used as tables (statics, pools, arrays sized by a named constant).
xargs grep -n -E '\[[^]\[]*;\s*([A-Z][A-Z0-9_]*(::[A-Z0-9_]+)?|[0-9][0-9_]*)\s*\]' < "$OUT/rs-nomath.txt" 2>/dev/null \
  | grep -E 'UnsafeCell<\[|Atomic[A-Za-z0-9]+(<[^>]*>)?;|static|Cell<\[|Mutex<\[|Option<[^;]*;\s*[A-Z]|;\s*[A-Z][A-Z0-9_]*(_MAX|_CAP|_SLOTS|_LIMIT|_ENTRIES|_COUNT)\]' \
  | grep -v -E 'static [A-Z0-9_]+: \[(u8|u16|u32|i32|u64|&str)' > "$OUT/route2-arrays.txt"
xargs grep -n -E '\[[0-9]{2,}\]|\[[A-Z_]{3,}\]\s*[;=]' < "$OUT/cfiles.txt" | grep -v -E '\.S:' >> "$OUT/route2-arrays.txt"

# Route 3: narrowing conversions of counts and indices.
xargs grep -n -E '\b(u8|u16|i8|i16)::try_from\(' < "$OUT/rs-nomath.txt" 2>/dev/null \
  | grep -v -E '^\S+:\s*//|assert' > "$OUT/route3-narrowing.txt"

# Route 4: error paths meaning "too many / no room": message strings, then invented errnos.
cat "$OUT/rs-nomath.txt" "$OUT/cfiles.txt" | xargs grep -n -i -E '"[^"]*(too many|too long|too deep|limit|no room|out of room|cannot load more|table full|exhausted)[^"]*"' 2>/dev/null \
  | grep -v -E '^\S+:\s*//' > "$OUT/route4-messages.txt"
xargs grep -n -E '\b(EMFILE|ENFILE|ELIBMAX|E2BIG|ENOSPC|ETOOMANYREFS|EUSERS|MM_NOTOK)\b' < "$OUT/rs-nomath.txt" 2>/dev/null \
  | grep -v -E 'errno_list|core/src/diag.rs|^\S+:\s*//|assert' > "$OUT/route4-errnos.txt"

# Route 5: depth, retry, iteration and time limits.
xargs grep -n -E '^\s*(pub(\([a-z]+\))?\s+)?const\s+[A-Z0-9_]*(RETR|TRIES|ATTEMPT|SPIN|ROUNDS|TIMEOUT|DEADLINE|BUDGET|ITER|STEPS|HOPS|LOOPS|NEST|RECUR|DEPTH)[A-Z0-9_]*\s*:' \
  < "$OUT/rs-nomath.txt" > "$OUT/route5-depth-retry.txt"
xargs grep -n -E 'depth\s*>=?\s*[A-Z_]+|> MAX_DEPTH' < "$OUT/rs-nomath.txt" >> "$OUT/route5-depth-retry.txt"

wc -l "$OUT"/route*.txt
echo "results in $OUT"
