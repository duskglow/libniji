//! Function declarations: the prototype (prototypes.tsv, or the resolved type of typedefs.tsv), `noexcept`, and the
//! symbol each configuration's call site references (callsites.tsv), checked against the default-version exports.
use super::*;

impl Gen<'_> {
    pub(super) fn declaration(&mut self, name: &str) -> Option<String> {
        // A function whose prototype the reference's own header leaves unrecorded (its parameter is a tag only
        // the header names) has its resolved type in typedefs.tsv (`int (int, sgttyb*)`): spelled as a prototype.
        let from_type = self.t.typedefs.get(&("function".to_string(), name.to_string())).filter(|_| !self.t.protos.contains_key(name)).and_then(|r| {
            let (ret, args) = r["cxx"].split_once(" (")?;
            let args = args.strip_suffix(')')?;
            // A pointer to a constant is spelled `const T *`.
            let args: Vec<String> = self.c_words(args).split(", ").map(|a| a.strip_suffix(" const *").map_or(a.to_string(), |t| format!("const {t} *"))).collect();
            Some(format!("{} {name} ({})", self.c_words(ret), args.join(", ")))
        });
        let Some(d) = from_type.as_ref().or_else(|| self.t.protos.get(name)) else {
            self.errors.push(format!("{}: function {name} not in prototypes.tsv", self.header));
            return None;
        };
        let d = d.strip_prefix("extern ").unwrap_or(d);
        let mut out = String::new();
        let mut word = String::new();
        let mut tags = Vec::new();
        let mut prev = String::new();
        for c in d.chars().chain(std::iter::once(' ')) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            if !word.is_empty() {
                if prev == "struct" || prev == "union" {
                    tags.push(format!("{prev} {word};"));
                }
                // `complex` is <complex.h>'s macro for the keyword (C 7.3.1): a declaration spells the keyword.
                let w = if word == name {
                    word.clone()
                } else if word == "complex" {
                    "_Complex".to_string()
                } else {
                    self.resolve(&word)
                };
                if !scalar(&w) && w == word && omitted(&self.t.reserved, &w) && self.t.typedefs.contains_key(&("typedef".to_string(), w.clone())) {
                    self.typedef(&w);
                }
                out.push_str(&w);
                prev = std::mem::take(&mut word);
            }
            out.push(c);
        }
        for t in tags {
            self.out.push(t);
        }
        // The decayed form of the compiler's `va_list` (an array of one `__va_list_tag`).
        Some(out.trim_end().replace("__va_list_tag *", "__builtin_va_list").replace("__va_list_tag*", "__builtin_va_list"))
    }

    /// The declaration of `name` binding to `sym`. A function that binds to its own name is redirected to its 64-bit twin under
    /// `_FILE_OFFSET_BITS=64` where the call probes saw that (`lfs`); the addresses probed in no GNU configuration cannot show it.
    /// `excl`: the branch's own condition already excludes `_FILE_OFFSET_BITS=64` (a plain conjunction with `!__NIJI_FOB64`).
    pub(super) fn extern_lines(&self, name: &str, decl: &str, nt: &str, sym: &str, ind: &str, excl: bool) -> Vec<String> {
        // The attributes (slice 19) come after the assembler label (GCC's declarator order).
        let at = self.t.attrs.get(name).map(String::as_str).unwrap_or("");
        // A libmvec function bound to its own name (the variants' names are the symbol's) is declared simd
        // (D-X.170); `__NIJI_SIMD` is empty unless the header defines it for a vectorizing configuration.
        let simd = if sym == name && self.t.simd.contains(name) { " __NIJI_SIMD" } else { "" };
        match self.t.lfs.get(name) {
            Some(y) if sym == name && !excl && self.t.exported.contains(y) => vec![
                format!("{ind}if __NIJI_FOB64"),
                format!("extern {decl}{nt} __asm__ (\"{y}\"){at};"),
                format!("{ind}else"),
                format!("extern {decl}{nt}{at}{simd};"),
                format!("{ind}endif"),
            ],
            _ if sym != name => vec![format!("extern {decl}{nt} __asm__ (\"{sym}\"){at};")],
            _ => vec![format!("extern {decl}{nt}{at}{simd};")],
        }
    }

    /// diagattrs.tsv noreturn `nonzero:K` (error, error_at_line: error(3) returns when the status is 0): the declaration
    /// carries no noreturn; when GCC optimizes, an always-inline wrapper calls a noreturn alias of the same symbol only
    /// for a nonzero constant argument K, so a status-0 or variable call is compiled as returning (D-2.13, ERROR-H-NORETURN).
    /// Clang and C++ (no `__builtin_va_arg_pack` in clang) get the plain declaration.
    fn nonzero_wrapper(&mut self, name: &str, decl: &str, nt: &str, sym: &str, k: usize) -> Vec<String> {
        let at = self.t.attrs.get(name).map(String::as_str).unwrap_or("");
        let parsed = decl.strip_prefix("void ").and_then(|d| d.strip_prefix(&format!("{name} ("))).and_then(|p| p.strip_suffix(')'));
        let Some(params) = parsed.map(|p| p.split(", ").collect::<Vec<_>>()).filter(|ps| ps.len() >= k && ps.iter().all(|p| !p.contains('('))) else {
            self.errors.push(format!("{}: {name}: noreturn nonzero:{k} needs a void function of simple parameters, not `{decl}`", self.header));
            return Vec::new();
        };
        let fixed: Vec<&str> = params.iter().copied().filter(|p| *p != "...").collect();
        let variadic = fixed.len() < params.len();
        let named: Vec<String> = fixed.iter().enumerate().map(|(i, p)| if p.ends_with('*') { format!("{p}__p{i}") } else { format!("{p} __p{i}") }).collect();
        let mut args: Vec<String> = (0..fixed.len()).map(|i| format!("__p{i}")).collect();
        if variadic {
            args.push("__builtin_va_arg_pack ()".into());
        }
        let (named, args, params) = (named.join(", ") + if variadic { ", ..." } else { "" }, args.join(", "), params.join(", "));
        let at_nr = match at.strip_suffix("))") {
            Some(a) => format!("{a}, __noreturn__))"),
            None => " __attribute__ ((__noreturn__))".into(),
        };
        let p = format!("__p{}", k - 1);
        vec![
            "#  if defined __OPTIMIZE__ && defined __GNUC__ && !defined __clang__ && !defined __cplusplus".into(),
            format!("extern void __niji_nr_{name} ({params}){nt} __asm__ (\"{sym}\"){at_nr};"),
            format!("extern void __niji_rt_{name} ({params}){nt} __asm__ (\"{sym}\"){at};"),
            format!("extern __inline __attribute__ ((__always_inline__, __gnu_inline__, __artificial__)) void {name} ({named}){nt}"),
            "{".into(),
            format!("  if (__builtin_constant_p ({p}) && {p} != 0)"),
            format!("    __niji_nr_{name} ({args});"),
            "  else".into(),
            format!("    __niji_rt_{name} ({args});"),
            "}".into(),
            "#  endif".into(),
        ]
    }

    pub(super) fn function(&mut self, name: &str) {
        let Some(decl) = self.declaration(name) else { return };
        let nt = if self.t.nothrow.contains(name) { " __NIJI_NOTHROW" } else { "" };
        // A configuration in which the probe's own `&f` did not compile (`!error`) says nothing about the symbol, and a
        // C++-mangled name that no library exports is a declaration without C linkage: libniji declares every function
        // `extern "C"`, so the plain name is the symbol (docs/differences-from-glibc.md).
        let syms: Vec<(String, String)> = self.t.addr.get(name).cloned().unwrap_or_default().into_iter().filter(|(_, s)| s != "!error" && !(s.starts_with("_Z") && !self.t.exported.contains(s))).collect();
        let builtin = self.t.calls.get(name).map(|s| s == "-").unwrap_or(false);
        let distinct: BTreeSet<&String> = syms.iter().map(|(_, s)| s).collect();
        let mut body = Vec::new();
        if distinct.len() <= 1 {
            let sym = distinct.into_iter().next().cloned().unwrap_or_else(|| name.to_string());
            // A function headers/providers.tsv gives to the program, the debugger or no library is declared all the same.
            if !builtin && !self.t.exported.contains(&sym) && !self.t.linked_elsewhere.contains(sym.as_str()) && !(sym == name && self.t.providers.contains_key(name)) {
                self.errors.push(format!("{}: {name} references {sym}, not exported at a default version", self.header));
            }
            body.extend(self.extern_lines(name, &decl, nt, &sym, "#  ", false));
            if let Some(&k) = self.t.nonzero_noreturn.get(name) {
                body.extend(self.nonzero_wrapper(name, &decl, nt, &sym, k));
            }
        } else if self.t.nonzero_noreturn.contains_key(name) {
            self.errors.push(format!("{}: {name}: noreturn nonzero:K with more than one symbol is not supported", self.header));
        } else {
            let all = (1u64 << self.t.lv.ids.len()) - 1;
            // `addr` probes run in no fortify configuration (callsites.tsv): those do not decide the symbol.
            let care = all & !self.t.lv.reject & !self.fortify_mask();
            for (i, (configs, sym)) in syms.iter().enumerate() {
                if !self.t.exported.contains(sym) {
                    self.errors.push(format!("{}: {name} references {sym}, not exported at a default version", self.header));
                }
                let m = mask_of(&self.t.lv, configs).unwrap_or(0);
                let c = condition(&self.t.lv, m, care).unwrap_or_else(|| {
                    self.errors.push(format!("{}: {name}: no condition selects {configs}", self.header));
                    "0".into()
                });
                body.push(format!("#  {} {c}", if i == 0 { "if" } else { "elif" }));
                body.extend(self.extern_lines(name, &decl, nt, sym, "#   ", c.contains("!__NIJI_FOB64") && !c.contains("||")));
            }
            body.push("#  endif".into());
        }
        // `_Bool` (C 6.2.5; stdbit.h's stdc_has_single_bit_*) is no C++ keyword: there the same type is `bool`.
        let body = body.into_iter().flat_map(|l| if l.starts_with("extern ") && l.contains("_Bool") { vec!["#  ifdef __cplusplus".into(), l.replace("_Bool", "bool"), "#  else".into(), l, "#  endif".into()] } else { vec![l] }).collect();
        self.guard(name, body);
    }
}
