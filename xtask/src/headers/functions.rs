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
            // The one such function family (gtty, stty) takes a pointer to a tag, `T*` or `T const*`.
            let args: Vec<String> = args
                .split(", ")
                .map(|a| match (a.strip_suffix(" const*"), a.strip_suffix('*')) {
                    (Some(t), _) => format!("const struct {t} *"),
                    (_, Some(t)) => format!("struct {t} *"),
                    _ => a.to_string(),
                })
                .collect();
            Some(format!("{ret} {name} ({})", args.join(", ")))
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
            if !builtin && !self.t.exported.contains(&sym) && !self.t.linked_elsewhere.contains(sym.as_str()) {
                self.errors.push(format!("{}: {name} references {sym}, not exported at a default version", self.header));
            }
            let asm = if sym != name { format!(" __asm__ (\"{sym}\")") } else { String::new() };
            body.push(format!("extern {decl}{nt}{asm};"));
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
                let asm = if sym != name { format!(" __asm__ (\"{sym}\")") } else { String::new() };
                body.push(format!("extern {decl}{nt}{asm};"));
            }
            body.push("#  endif".into());
        }
        self.guard(name, body);
    }
}
