//! Typedefs from typedefs.tsv: scalar, qualified, array, function and function-pointer types, aggregates with or
//! without a layout, and typedefs named for linkage through a structure tag.
use super::*;

impl Gen<'_> {
    pub(super) fn typedef(&mut self, name: &str) {
        let Some(r) = self.t.typedefs.get(&("typedef".to_string(), name.to_string())).cloned() else {
            self.errors.push(format!("{}: typedef {name} not in typedefs.tsv", self.header));
            return;
        };
        let cxx = r["cxx"].clone();
        // A qualified scalar (pthread_spinlock_t is `int volatile`) is still a scalar typedef.
        let body = if cxx.is_empty() && name == "wchar_t" {
            // A keyword in C++ (typedefs.tsv leaves its spelling empty); in C the compiler's type.
            vec!["#  ifndef __cplusplus".into(), format!("typedef __WCHAR_TYPE__ {name};"), "#  endif".into()]
        } else if scalar(&cxx) || scalar(cxx.replace("volatile", "").replace("const", "").trim()) {
            vec![format!("typedef {} {name};", cxx.replace('*', " *"))]
        } else if cxx.ends_with(']') && !cxx.contains('(') {
            // An array type (`elf_gregset_t` is `unsigned long long [27]`).
            let i = cxx.find('[').unwrap_or(0);
            vec![format!("typedef {} {name}{};", cxx[..i].trim(), &cxx[i..])]
        } else if let Some(open) = cxx.find('(') {
            // A function or function-pointer type (`int (*)(void const*, void const*)` or `int (void*)`).
            let (ret, rest) = cxx.split_at(open);
            let args = self.type_words(rest);
            let ret = self.type_words(ret);
            match args.strip_prefix("(*)") {
                Some(a) => vec![format!("typedef {} (*{name}){a};", ret.trim_end())],
                None => vec![format!("typedef {} {name}{args};", ret.trim_end())],
            }
        } else if cxx == name || cxx.is_empty() {
            match self.members_nested(name) {
                Some((kw, m, nested)) => {
                    self.emit_nested(nested);
                    [vec![format!("typedef {kw} {{")], m, vec![format!("}} {name};")]].concat()
                }
                None => vec![format!("typedef struct {{ char __niji_opaque[{}] __attribute__ ((__aligned__ ({}))); }} {name};", r["size"], r["align"])],
            }
        } else if self.t.typedefs.contains_key(&("typedef".to_string(), cxx.clone())) {
            // Named for linkage by another typedef (C++ mangles through it): that one comes first.
            self.typedef(&cxx);
            vec![format!("typedef {cxx} {name};")]
        } else if !cxx.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // A pointer to a structure (`locale_t` is `struct __locale_struct *`): the tag stays incomplete.
            vec![format!("typedef struct {} {name};", cxx.replace('*', " *"))]
        } else {
            // The structure is named for linkage by `cxx` (`fpos_t` is `struct _G_fpos_t`): define it here.
            let def = match self.members_nested(name) {
                Some((kw, m, nested)) => {
                    self.emit_nested(nested);
                    [vec![format!("{kw} {cxx} {{")], m, vec!["};".into()]].concat()
                }
                None => vec![format!("struct {cxx} {{ char __niji_opaque[{}] __attribute__ ((__aligned__ ({}))); }};", r["size"], r["align"])],
            };
            let kw = if def[0].starts_with("union") { "union" } else { "struct" };
            // The same guard as a `struct` row of visibility.tsv for this tag (tag()), so it is defined once.
            self.guard(&format!("struct_{cxx}"), def);
            vec![format!("typedef {kw} {cxx} {name};")]
        };
        self.guard(name, body);
    }

    /// The C++ form of a `typedefs.tsv` typedef, or an error.
    pub(super) fn cxx(&self, name: &str) -> Option<String> {
        self.t.typedefs.get(&("typedef".to_string(), name.to_string())).map(|r| r["cxx"].clone())
    }
}
