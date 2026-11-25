//! Slice 18 (plan/headers.md section 4: `_FORTIFY_SOURCE` levels 1 to 3). Listed by `xtask/build.rs`.
//!
//! * `@@fortify`: after the header's `@@emit`, the fortify wrapper of every function that `@@emit` declared in this
//!   header and `headers/fortify.tsv` lists (libniji's wrapper design), under the function's own condition and
//!   `__NIJI_FORTIFY >= L`, L the lowest level at which `spec/abi/callsites.tsv` shows its call reaching a `_chk`
//!   symbol. The `_chk` declarations come from `spec/abi/chk-prototypes.tsv` (Appendix C.9); the object sizes from
//!   GCC's object-size builtins (GCC manual, Object Size Checking). A wrapper's plain call goes through an
//!   assembler-labelled alias of the function's own symbol: a call to the wrapped name inside its own inline body
//!   is a recursive call to GCC (an endless loop at -O2), not a call of the library's definition.
use super::super::{omitted, scalar, tsv, Gen, Marker, Slice};
use std::collections::BTreeMap;

pub(super) const SLICE: Slice = Slice { markers: &[Marker { name: "@@fortify", run: fortify }], inline: &[], linked_elsewhere: &[] };

const TABLE: &str = "headers/fortify.tsv";

/// The definitions every wrapper uses, once per translation unit.
const DEFS: &[&str] = &[
    "#ifndef __NIJI_FORTIFY_DEFS",
    "# define __NIJI_FORTIFY_DEFS 1",
    "# define __NIJI_FORTIFY_FN extern __inline __attribute__ ((__always_inline__, __gnu_inline__, __artificial__))",
    "# define __NIJI_FORTIFY_FLAG (__NIJI_FORTIFY > 1)",
    "# if __NIJI_FORTIFY > 2 && defined __has_builtin",
    "#  if __has_builtin (__builtin_dynamic_object_size)",
    "#   define __NIJI_BOS(p, t) __builtin_dynamic_object_size (p, t)",
    "#  endif",
    "# endif",
    "# ifndef __NIJI_BOS",
    "#  define __NIJI_BOS(p, t) __builtin_object_size (p, t)",
    "# endif",
    "#endif",
];

fn fortify(g: &mut Gen<'_>, _args: &[&str]) {
    let load = |rel: &str| tsv(g.root, rel);
    let (rows, calls, chk) = match (load(TABLE), load("spec/abi/callsites.tsv"), load("spec/abi/chk-prototypes.tsv")) {
        (Ok(a), Ok(b), Ok(c)) => (a, b, c),
        (a, b, c) => {
            g.errors.extend([a.err(), b.err(), c.err()].into_iter().flatten());
            return;
        }
    };
    let chk: BTreeMap<String, String> = chk.into_iter().map(|r| (r["name"].clone(), r["declaration"].clone())).collect();
    let mut defs = false;
    let mine: Vec<&BTreeMap<String, String>> = rows.iter().filter(|r| r["form"] != "macro" && g.emitted.contains(&format!("function {}", r["name"]))).collect();
    for r in mine {
        let name = r["name"].as_str();
        // (configs, symbol) of each `_chk` symbol a call reaches; vprintf's row names `__vfprintf_chk,stdout`.
        let syms: Vec<(String, String)> = calls
            .iter()
            .filter(|c| c["name"] == name && (c["mode"] == "call" || c["mode"] == "macro"))
            .map(|c| (c["configs"].clone(), c["symbol"].split(',').next().unwrap_or("").to_string()))
            .filter(|(_, s)| s.ends_with("_chk") || (r["form"].starts_with("mode") && s.ends_with("_2")))
            .collect();
        let mut levels = [false; 4];
        let mut variants = [false; 3];
        for (configs, _) in &syms {
            for (l, v) in call_levels(configs) {
                levels[l] = true;
                variants[v] = true;
            }
        }
        // The call probes' GNU configurations alone reach the `_chk` symbol (mempcpy, asprintf): the wrapper is GNU's.
        let gnu = match variants {
            [true, _, _] => "",
            [false, true, true] => " && __NIJI_GNU",
            _ => {
                g.errors.push(format!("{}: {name}: the _chk call configurations {syms:?} are neither all nor the GNU ones", g.header));
                continue;
            }
        };
        let Some(lmin) = (1..=3).find(|l| levels[*l]) else {
            g.errors.push(format!("{}: {TABLE}: {name}: no callsites.tsv call reaches a _chk symbol", g.header));
            continue;
        };
        if (lmin..=3).any(|l| !levels[l]) {
            g.errors.push(format!("{}: {name}: the _chk call is not made at every level from {lmin} to 3", g.header));
            continue;
        }
        // Two symbols: the one every configuration of which is `_FILE_OFFSET_BITS=64` (pread: `__pread64_chk`).
        let lfs = |c: &str| c.split(',').all(|x| x.ends_with("+gnu64") && !x.starts_with('!'));
        let branches: Vec<(Option<&str>, String, Option<bool>)> = match syms.as_slice() {
            [(_, s)] => vec![(None, s.clone(), None)],
            [(a, x), (b, y)] if lfs(a) != lfs(b) => {
                let (big, small) = if lfs(a) { (x, y) } else { (y, x) };
                vec![(Some("#  if __NIJI_FOB64"), big.clone(), Some(true)), (Some("#  else"), small.clone(), Some(false))]
            }
            _ => {
                g.errors.push(format!("{}: {name}: callsites.tsv gives _chk symbols no rule selects: {syms:?}", g.header));
                continue;
            }
        };
        if !defs {
            g.out.push("#if __NIJI_FORTIFY > 0".into());
            g.out.extend(DEFS.iter().map(|l| l.to_string()));
            g.out.push("#endif".into());
            defs = true;
        }
        let c = g.cond(name, Some("function"));
        let mut body = Vec::new();
        for (head, sym, fob) in &branches {
            body.extend(head.map(String::from));
            body.extend(wrapper(g, r, sym, &chk, *fob));
        }
        if branches.len() > 1 {
            body.push("#  endif".into());
        }
        g.out.push(format!("#if ({c}){gnu} && __NIJI_FORTIFY >= {lmin}"));
        g.out.extend([format!("# ifndef __NIJI_F_{name}"), format!("#  define __NIJI_F_{name} 1")]);
        g.out.extend(body);
        g.out.extend(["# endif".to_string(), "#endif".to_string()]);
    }
}

/// The fortify levels of a callsites.tsv call row's configurations. The call probes run in their own configurations
/// (`fortify<L>`, `fortify<L>+gnu`, `fortify<L>+gnu64`; `*` all of them, `!` the others of them): (level, variant),
/// the variant 0 plain, 1 `+gnu`, 2 `+gnu64`.
fn call_levels(configs: &str) -> Vec<(usize, usize)> {
    let all: Vec<String> = (1..=3).flat_map(|l| ["", "+gnu", "+gnu64"].map(|s| format!("fortify{l}{s}"))).collect();
    let picked: Vec<&String> = match configs.strip_prefix('!') {
        _ if configs == "*" => all.iter().collect(),
        Some(rest) => all.iter().filter(|c| !rest.split(',').any(|x| x == c.as_str())).collect(),
        None => all.iter().filter(|c| configs.split(',').any(|x| x == c.as_str())).collect(),
    };
    picked
        .iter()
        .filter_map(|c| {
            let rest = c.strip_prefix("fortify")?;
            let v = if rest.ends_with("+gnu64") { 2 } else { usize::from(rest.ends_with("+gnu")) };
            rest.chars().next()?.to_digit(10).map(|d| (d as usize, v))
        })
        .collect()
}

/// The declaration of `name` split at its own name: (return type with its trailing space or `*`, parameter types).
fn split_decl(decl: &str, name: &str) -> Option<(String, Vec<String>)> {
    let key = format!("{name} (");
    let at = decl.match_indices(&key).map(|(i, _)| i).find(|&i| i == 0 || !decl[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_'))?;
    let inner = decl[at + key.len()..].strip_suffix(')')?;
    let (mut params, mut depth, mut cur) = (Vec::new(), 0, String::new());
    for ch in inner.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                params.push(std::mem::take(&mut cur).trim().to_string());
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    params.push(cur.trim().to_string());
    params.retain(|p| !p.is_empty() && p != "void");
    Some((decl[..at].to_string(), params))
}

/// One `$` token of `fortify.tsv` in C (`$N`, `$SN`, `$EN`, `$F`, `$V`); `ost` is the row's object-size type.
fn expand(text: &str, ost: &str) -> String {
    let t = if ost == "0" { "0".to_string() } else { "(__NIJI_FORTIFY > 1)".to_string() };
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('$') {
        out.push_str(&rest[..i]);
        rest = &rest[i + 1..];
        let kind = rest.chars().next().unwrap_or(' ');
        let (kind, after) = if matches!(kind, 'S' | 'E' | 'F' | 'V') { (Some(kind), &rest[1..]) } else { (None, rest) };
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        rest = &after[digits.len()..];
        out.push_str(&match kind {
            Some('F') => "__NIJI_FORTIFY_FLAG".to_string(),
            Some('V') => "__builtin_va_arg_pack ()".to_string(),
            Some('S') => format!("__NIJI_BOS (__p{digits}, {t})"),
            Some('E') => format!("(__NIJI_BOS (__p{digits}, {t}) / sizeof (*__p{digits}))"),
            _ => format!("__p{digits}"),
        });
    }
    out + rest
}

/// A table declaration in C: each word through the generator's type resolution, as `Gen::declaration` spells a
/// prototypes.tsv row (reserved typedefs declared first; the compiler's `va_list` in its decayed form).
fn spell(g: &mut Gen<'_>, decl: &str, name: &str) -> String {
    let d = decl.strip_prefix("extern ").unwrap_or(decl);
    let (mut out, mut word) = (String::new(), String::new());
    for c in d.chars().chain(std::iter::once(' ')) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            let w = if word == name { word.clone() } else { g.resolve(&word) };
            if !scalar(&w) && w == word && omitted(&g.t.reserved, &w) && g.t.typedefs.contains_key(&("typedef".to_string(), w.clone())) {
                g.typedef(&w);
            }
            out.push_str(&w);
            word.clear();
        }
        out.push(c);
    }
    out.trim_end().replace("__va_list_tag *", "__builtin_va_list").replace("__va_list_tag*", "__builtin_va_list")
}

/// The lines of one wrapper of `r["name"]` reaching `sym`.
fn wrapper(g: &mut Gen<'_>, r: &BTreeMap<String, String>, sym: &str, chk: &BTreeMap<String, String>, fob: Option<bool>) -> Vec<String> {
    let name = r["name"].as_str();
    let nt = if g.t.nothrow.contains(name) { " __NIJI_NOTHROW" } else { "" };
    let Some(decl) = g.declaration(name) else { return Vec::new() };
    let Some((ret, params)) = split_decl(&decl, name) else {
        g.errors.push(format!("{}: {name}: cannot split the declaration {decl:?}", g.header));
        return Vec::new();
    };
    if r["form"] == "redirect" {
        let at = g.t.attrs.get(name).cloned().unwrap_or_default();
        return vec![format!("extern {decl}{nt} __asm__ (\"{sym}\"){at};")];
    }
    if params.iter().any(|p| p.contains('(') || p.contains('[')) {
        g.errors.push(format!("{}: {name}: a parameter the wrapper cannot name: {decl:?}", g.header));
        return Vec::new();
    }
    let named: Vec<String> = params.iter().enumerate().map(|(i, p)| if p == "..." { p.clone() } else if p.ends_with('*') { format!("{p}__p{i}") } else { format!("{p} __p{i}") }).collect();
    let void = ret.trim() == "void";
    let (args, ost) = (r["args"].as_str(), r["ost"].as_str());
    let mut out = Vec::new();
    let call = match r["form"].as_str() {
        "builtin" => expand(args, ost),
        "va" | "chk" | "mode" | "mode2" => {
            let Some(proto) = chk.get(sym) else {
                g.errors.push(format!("{}: {name}: {sym} has no row in spec/abi/chk-prototypes.tsv", g.header));
                return Vec::new();
            };
            let d = spell(g, proto, sym);
            out.push(format!("#  ifndef __NIJI_D_{sym}"));
            out.push(format!("#   define __NIJI_D_{sym} 1"));
            out.push(format!("extern {d}{nt};"));
            out.push("#  endif".into());
            format!("{sym} ({})", expand(args, ost))
        }
        f => {
            g.errors.push(format!("{}: {TABLE}: {name}: unknown form {f}", g.header));
            return Vec::new();
        }
    };
    let ret_kw = if void { "" } else { "return " };
    let mut fn_body = Vec::new();
    // `mode` takes one optional argument (the open family's mode), `mode2` two passed together (mq_open's mode and
    // attributes, spec 25.3); the rest of the two forms is the same.
    let optional = match r["form"].as_str() {
        "mode" => 1,
        "mode2" => 2,
        _ => 0,
    };
    if r["form"] == "chk" || optional > 0 {
        // The plain call where the destination's size is unknown or the call provably fits.
        let dest = args.split(['$']).skip(1).find(|t| t.starts_with('S') || t.starts_with('E')).map(|t| t[1..].chars().take_while(char::is_ascii_digit).collect::<String>());
        let mut tests = Vec::new();
        if let Some(n) = dest {
            tests.push(format!("{} == (__SIZE_TYPE__) -1", expand(&format!("$S{n}"), ost)));
        }
        if r["safe"] != "-" {
            let s = expand(&r["safe"], ost);
            tests.push(format!("(__builtin_constant_p ({s}) && ({s}))"));
        }
        let plain = format!("__niji_plain_{name}");
        // The function's own symbol, as its declaration binds it: its 64-bit twin under `_FILE_OFFSET_BITS=64`
        // where the address probes saw one (pread -> pread64).
        // Spelled from the prototype, not `__typeof__ (name)`: that names the function, and a deprecated one (getwd) warns.
        let alias = |s: &str| format!("extern {ret}{plain} ({}){nt} __asm__ (\"{s}\");", params.join(", "));
        let twin = g.t.lfs.get(name).cloned().or_else(|| {
            let other: Vec<&String> = g.t.addr.get(name).into_iter().flatten().map(|(_, s)| s).filter(|s| *s != name && *s != "!error" && g.t.exported.contains(*s)).collect();
            (other.len() == 1).then(|| other[0].clone())
        });
        match (twin, fob) {
            (Some(y), Some(true)) => out.push(alias(&y)),
            (Some(y), None) => out.extend(["#  if __NIJI_FOB64".to_string(), alias(&y), "#  else".to_string(), alias(name), "#  endif".to_string()]),
            _ => out.push(alias(name)),
        }
        // A variadic function passes its own variable arguments on (GCC: `__builtin_va_arg_pack`).
        let pass: Vec<String> = params.iter().enumerate().map(|(i, p)| if p == "..." { "__builtin_va_arg_pack ()".to_string() } else { format!("__p{i}") }).collect();
        if optional > 0 {
            // The open family (spec 11.1) and mq_open (spec 25.3): GCC's `__builtin_va_arg_pack_len` counts the
            // variable arguments, and a call left in to a function with the `error` attribute fails the compile (GCC
            // manual, Function Attributes).
            let need = expand(&r["safe"], ost);
            // The plain call only for flags that are themselves a constant: flags built at run time from constants
            // (`f = O_RDONLY; if (x) f |= O_NONBLOCK;`) let GCC fold `need` while the flags stay unknown (own probe).
            let flags = r["safe"].split('$').nth(1).map(|t| t.chars().take_while(char::is_ascii_digit).collect::<String>()).unwrap_or_default();
            let flags = expand(&format!("${flags}"), ost);
            let fixed: Vec<&String> = pass.iter().filter(|p| !p.starts_with("__builtin")).collect();
            let fixed = fixed.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            let (sfx, need_msg, args_msg, bad) = if optional == 1 {
                ("", "the flags need a mode argument (O_CREAT or O_TMPFILE)", "too many arguments: one mode at most", "> 1".to_string())
            } else {
                ("2", "O_CREAT needs the mode and attributes arguments", "pass the mode and the attributes together, or neither", "!= 0 && __builtin_va_arg_pack_len () != 2".to_string())
            };
            out.push(format!("#  ifndef __NIJI_D_fortify_mode{sfx}"));
            out.push(format!("#   define __NIJI_D_fortify_mode{sfx} 1"));
            out.push(format!("extern void __niji_fortify_mode{sfx} (void) __attribute__ ((__error__ (\"{need_msg}\")));"));
            out.push(format!("extern void __niji_fortify_args{sfx} (void) __attribute__ ((__error__ (\"{args_msg}\")));"));
            out.push("#  endif".into());
            fn_body.push(format!("  if (__builtin_va_arg_pack_len () {bad})"));
            fn_body.push(format!("    __niji_fortify_args{sfx} ();"));
            fn_body.push("  if (__builtin_va_arg_pack_len () < 1)".into());
            fn_body.push("    {".into());
            fn_body.push(format!("      if (__builtin_constant_p ({flags}))"));
            fn_body.push("        {".into());
            fn_body.push(format!("          if ({need})"));
            fn_body.push(format!("            __niji_fortify_mode{sfx} ();"));
            fn_body.push(format!("          return {plain} ({fixed});"));
            fn_body.push("        }".into());
            fn_body.push(format!("      return {call};"));
            fn_body.push("    }".into());
            fn_body.push(format!("  return {plain} ({});", pass.join(", ")));
        } else if !tests.is_empty() {
            fn_body.push(format!("  if ({})", tests.join(" || ")));
            fn_body.push(if void { format!("    {{ {plain} ({}); return; }}", pass.join(", ")) } else { format!("    return {plain} ({});", pass.join(", ")) });
        }
    }
    if optional == 0 {
        fn_body.push(format!("  {ret_kw}{call};"));
    }
    out.push(format!("__NIJI_FORTIFY_FN {ret}{name} ({}){nt}", named.join(", ")));
    out.push("{".into());
    out.extend(fn_body);
    out.push("}".into());
    out
}

#[cfg(test)]
mod tests {
    use super::super::super::generate;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    /// A program that writes `argv[2]` bytes (not a compile-time constant) into an 8-byte object through
    /// `argv[1]`: read from /dev/zero, memcpy, or (dynamic size, level 3 only) memset of malloc (8).
    const PROGRAM: &str = r#"#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
int main (int argc, char **argv)
{
  char b[8];
  static const char src[64];
  size_t n = (size_t) atoi (argv[2]);
  if (argc < 3)
    return 2;
  if (argv[1][0] == 'r')
    {
      int fd = open ("/dev/zero", O_RDONLY);
      if (read (fd, b, n) < 0)
        return 3;
    }
  else if (argv[1][0] == 'm')
    memcpy (b, src, n);
  else
    {
      char *p = malloc (8);
      memset (p, 0, n);
      b[0] = p[0];
      free (p);
    }
  return b[0];
}
"#;

    // Appendix C.9 (`_chk` argument shapes) and the GCC manual, Object Size Checking: built at -O2 with libniji's
    // headers, a write of exactly the object's size runs, and one byte more ends the process with SIGABRT through the
    // library's `_chk` check (the size each wrapper passes is the object's own, not one off). Linked and run with the
    // build host's C library, which exports the same `_chk` ABI; libniji's own `__chk_fail` is its boundary crate's.
    // Skipped, and says why, where the compiler or the build host's C headers are absent.
    #[test]
    fn a_deliberate_overflow_aborts_and_an_exact_fit_runs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let gcc_ok = Command::new("gcc").arg("--version").output().is_ok_and(|o| o.status.success());
        if !gcc_ok || !Path::new("/usr/include/x86_64-linux-gnu").is_dir() {
            eprintln!("skipped: needs gcc and the build host's kernel headers (the generated headers include them)");
            return;
        }
        let (files, errors) = generate(&root).expect("generate");
        assert!(errors.is_empty(), "{errors:?}");
        let dir = std::env::temp_dir().join(format!("niji-fortify-test.{}", std::process::id()));
        for (h, text) in &files {
            let p = dir.join("inc").join(h);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, text).unwrap();
        }
        fs::write(dir.join("t.c"), PROGRAM).unwrap();
        let gccinc = String::from_utf8(Command::new("gcc").arg("-print-file-name=include").output().unwrap().stdout).unwrap();
        let mut failures = Vec::new();
        for level in ["1", "2", "3"] {
            let exe = dir.join(format!("t{level}"));
            let out = Command::new("gcc")
                .args(["-O2", &format!("-D_FORTIFY_SOURCE={level}"), "-nostdinc", "-isystem"])
                .arg(dir.join("inc"))
                .args(["-isystem", gccinc.trim(), "-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include", "-o"])
                .arg(&exe)
                .arg(dir.join("t.c"))
                .output()
                .expect("gcc");
            assert!(out.status.success(), "level {level}: {}", String::from_utf8_lossy(&out.stderr));
            for (how, dynamic) in [("read", false), ("memcpy", false), ("heap", true)] {
                if dynamic && level != "3" {
                    continue;
                }
                let run = |n: &str| Command::new(&exe).args([how, n]).output().expect("run");
                let (fit, over) = (run("8"), run("9"));
                use std::os::unix::process::ExitStatusExt;
                if fit.status.signal().is_some() {
                    failures.push(format!("level {level} {how}: 8 bytes into 8 ended by signal {:?}", fit.status.signal()));
                }
                if over.status.signal() != Some(6) {
                    failures.push(format!("level {level} {how}: 9 bytes into 8 did not abort ({:?})", over.status));
                }
            }
        }
        let _ = fs::remove_dir_all(&dir);
        assert!(failures.is_empty(), "{failures:#?}");
    }

    // Spec 11.1 (`__open_2`, `__openat_2`: fortify; the mode is required iff O_CREAT or O_TMPFILE) and
    // callsites.tsv's open-family call rows: compiled at -O2 with _FORTIFY_SOURCE=2 against libniji's headers, a call
    // without its mode and with run-time flags references the `_2` entry (the 64-bit one under
    // `_FILE_OFFSET_BITS=64`), a call with a mode or with constant flags that need none references the plain
    // function, and constant flags that need a mode, or two variable arguments, do not compile. Spec 25.3
    // (`__mq_open_2`) and mq_open's call row: the same with mq_open's two optional arguments (mode and attributes),
    // which a call passes together or not at all (one or three do not compile; probe-settled on the reference by
    // compile status and `nm -u`).
    #[test]
    fn an_open_without_its_mode_calls_the_checked_entry() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let tools = ["gcc", "nm"].iter().all(|t| Command::new(t).arg("--version").output().is_ok_and(|o| o.status.success()));
        if !tools || !Path::new("/usr/include/x86_64-linux-gnu").is_dir() {
            eprintln!("skipped: needs gcc, nm and the build host's kernel headers (the generated headers include them)");
            return;
        }
        let (files, errors) = generate(&root).expect("generate");
        assert!(errors.is_empty(), "{errors:?}");
        let dir = std::env::temp_dir().join(format!("niji-fortify-open.{}", std::process::id()));
        for (h, text) in &files {
            let p = dir.join("inc").join(h);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, text).unwrap();
        }
        let gccinc = String::from_utf8(Command::new("gcc").arg("-print-file-name=include").output().unwrap().stdout).unwrap();
        // (extra flags, call, the undefined symbol it must reference or None for a compile error, a symbol it must not)
        let cases: &[(&str, &str, Option<&str>, &str)] = &[
            ("", "open (p, f)", Some("__open_2"), "open"),
            ("", "open (p, f, 0600)", Some("open"), "__open_2"),
            ("", "open (p, O_RDONLY)", Some("open"), "__open_2"),
            ("", "open (p, d ? O_RDONLY | O_NONBLOCK : O_RDONLY)", Some("__open_2"), "open"),
            ("", "open (p, O_WRONLY | O_CREAT)", None, ""),
            ("-D_GNU_SOURCE", "open (p, O_WRONLY | O_TMPFILE)", None, ""),
            ("", "open (p, f, 0600, 1)", None, ""),
            ("", "openat (d, p, f)", Some("__openat_2"), "openat"),
            ("", "openat (d, p, f, 0600)", Some("openat"), "__openat_2"),
            ("-D_GNU_SOURCE -D_FILE_OFFSET_BITS=64", "open (p, f)", Some("__open64_2"), "__open_2"),
            ("-D_GNU_SOURCE -D_FILE_OFFSET_BITS=64", "open (p, f, 0600)", Some("open64"), "__open64_2"),
            ("-D_GNU_SOURCE", "openat64 (d, p, f)", Some("__openat64_2"), "openat64"),
            ("", "mq_open (p, f)", Some("__mq_open_2"), "mq_open"),
            ("", "mq_open (p, O_RDONLY)", Some("mq_open"), "__mq_open_2"),
            ("", "mq_open (p, f, 0600, (struct mq_attr *) 0)", Some("mq_open"), "__mq_open_2"),
            ("", "mq_open (p, O_RDWR | O_CREAT)", None, ""),
            ("-D_GNU_SOURCE", "mq_open (p, O_RDWR | O_TMPFILE)", Some("mq_open"), "__mq_open_2"),
            ("", "mq_open (p, f, 0600)", None, ""),
            ("", "mq_open (p, f, 0600, (struct mq_attr *) 0, 1)", None, ""),
        ];
        let mut failures = Vec::new();
        for (k, (extra, call, want, not)) in cases.iter().enumerate() {
            let src = dir.join(format!("o{k}.c"));
            let obj = dir.join(format!("o{k}.o"));
            fs::write(&src, format!("#include <fcntl.h>\n#include <mqueue.h>\nint lp_t (int d, const char *p, int f) {{ (void) d; (void) f; return {call}; }}\n")).unwrap();
            let out = Command::new("gcc")
                .args(["-O2", "-D_FORTIFY_SOURCE=2", "-c", "-nostdinc", "-isystem"])
                .arg(dir.join("inc"))
                .args(["-isystem", gccinc.trim(), "-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include"])
                .args(extra.split_whitespace())
                .arg("-o")
                .arg(&obj)
                .arg(&src)
                .output()
                .expect("gcc");
            match want {
                None if out.status.success() => failures.push(format!("{extra} {call}: compiled, wants a compile-time error")),
                None => {}
                Some(_) if !out.status.success() => failures.push(format!("{extra} {call}: {}", String::from_utf8_lossy(&out.stderr))),
                Some(w) => {
                    let nm = Command::new("nm").arg("-u").arg(&obj).output().expect("nm");
                    let syms: Vec<String> = String::from_utf8_lossy(&nm.stdout).split_whitespace().filter(|s| *s != "U").map(String::from).collect();
                    if !syms.iter().any(|s| s == w) || syms.iter().any(|s| s == not) {
                        failures.push(format!("{extra} {call}: references {syms:?}, wants {w} and not {not}"));
                    }
                }
            }
        }
        let _ = fs::remove_dir_all(&dir);
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
