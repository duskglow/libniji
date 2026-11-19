//! Typedefs from typedefs.tsv: scalar, qualified, array, function and function-pointer types, aggregates with or
//! without a layout, incomplete types (no size), the character types C++ has as keywords, and typedefs named for
//! linkage through a structure or union tag.
use super::*;

impl Gen<'_> {
    pub(super) fn typedef(&mut self, name: &str) {
        let Some(r) = self.t.typedefs.get(&("typedef".to_string(), name.to_string())).cloned() else {
            self.errors.push(format!("{}: typedef {name} not in typedefs.tsv", self.header));
            return;
        };
        let cxx = r["cxx"].clone();
        let opaque = |kw: &str| format!("{kw} {{ char __niji_opaque[{}] __attribute__ ((__aligned__ ({}))); }}", r["size"], r["align"]);
        // A qualified scalar (pthread_spinlock_t is `int volatile`) is still a scalar typedef.
        let body = if cxx.is_empty() && matches!(name, "wchar_t" | "char16_t" | "char32_t") {
            // A keyword in C++ (typedefs.tsv leaves its spelling empty; char16_t and char32_t from C++11); in C the
            // compiler's type (C11 7.19, 7.28).
            let ty = format!("__{}_TYPE__", name.trim_end_matches("_t").to_uppercase());
            let cond = if name == "wchar_t" { "#  ifndef __cplusplus" } else { "#  if !defined __cplusplus || __cplusplus < 201103L" };
            vec![cond.into(), format!("typedef {ty} {name};"), "#  endif".into()]
        } else if cxx.is_empty() && name == "char8_t" {
            // C23 7.30: `unsigned char`; a keyword in C++ where the compiler says so (`__cpp_char8_t`, C++20).
            vec!["#  if !defined __cplusplus || !defined __cpp_char8_t".into(), format!("typedef unsigned char {name};"), "#  endif".into()]
        } else if scalar(&cxx) || scalar(cxx.replace("volatile", "").replace("const", "").trim()) {
            vec![format!("typedef {} {name};", cxx.replace('*', " *"))]
        } else if cxx.ends_with(']') && !cxx.contains('(') {
            // An array type (`elf_gregset_t` is `unsigned long long [27]`; `jmp_buf` an array of one structure).
            let i = cxx.find('[').unwrap_or(0);
            let elem = cxx[..i].trim();
            if elem == "__va_list_tag" {
                // The compiler's `va_list` (psABI 3.5.7: an array of one `__va_list_tag`).
                vec![format!("typedef __builtin_va_list {name};")]
            } else if let Some(kind) = ["struct", "union"].into_iter().find(|k| self.tag_known(k, elem)) {
                self.tag(elem, kind);
                vec![format!("typedef {kind} {elem} {name}{};", &cxx[i..])]
            } else {
                vec![format!("typedef {elem} {name}{};", &cxx[i..])]
            }
        } else if let Some(open) = cxx.find('(') {
            // A function or function-pointer type (`int (*)(void const*, void const*)` or `int (void*)`).
            let (ret, rest) = cxx.split_at(open);
            let args = self.type_words(rest);
            let ret = self.type_words(ret);
            match args.strip_prefix("(*)") {
                Some(a) => vec![format!("typedef {} (*{name}){a};", ret.trim_end())],
                None => vec![format!("typedef {} {name}{args};", ret.trim_end())],
            }
        } else if name == "__gwchar_t" {
            // C 7.8.2.3 (see resolve()): the compiler's wchar_t type in C, wchar_t itself in C++ (features.h).
            vec![format!("typedef __NIJI_WCHAR {name};")]
        } else if let Some(kind) = ["struct", "union"].into_iter().find(|k| (cxx == name || cxx.is_empty()) && self.t.typedefs.contains_key(&(k.to_string(), name.to_string()))) {
            // The typedef and its tag share the name (`typedef struct ucontext_t {..} ucontext_t`): C++ has one class
            // of that name, so the typedef names the tag.
            self.tag(name, kind);
            vec![format!("typedef {kind} {name} {name};")]
        } else if cxx == name || cxx.is_empty() {
            match self.aggregate(name) {
                Some(a) => {
                    self.emit_nested(a.nested);
                    [vec![format!("typedef {}{} {{", a.kw, a.attrs)], a.lines, vec![format!("}} {name}{};", a.td_attrs)]].concat()
                }
                // No size: an incomplete type the program only points to.
                None if r["size"].is_empty() => vec![format!("typedef struct {name} {name};")],
                None => vec![format!("typedef {} {name};", opaque("struct"))],
            }
        } else if self.t.typedefs.contains_key(&("typedef".to_string(), cxx.clone())) {
            // Named for linkage by another typedef (C++ mangles through it): that one comes first.
            self.typedef(&cxx);
            vec![format!("typedef {cxx} {name};")]
        } else if !cxx.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // A pointer to a structure (`locale_t` is `struct __locale_struct *`): the tag stays incomplete.
            vec![format!("typedef struct {} {name};", cxx.replace('*', " *"))]
        } else {
            // The structure or union is named for linkage by `cxx` (`fpos_t` is `struct _G_fpos_t`, `__sigval_t`
            // `union sigval`): defined here, under the guard its tag row (tag()) uses, so it is defined once.
            let a = self.aggregate(name);
            let kind = ["union", "struct"].into_iter().find(|k| self.t.typedefs.contains_key(&(k.to_string(), cxx.clone()))).or(a.as_ref().map(|a| a.kw)).unwrap_or("struct");
            let mut td = String::new();
            let def = match a {
                Some(a) => {
                    self.emit_nested(a.nested);
                    td = a.td_attrs;
                    [vec![format!("{kind}{} {cxx} {{", a.attrs)], a.lines, vec!["};".into()]].concat()
                }
                // No size: an incomplete type the program only points to (`DIR` is `struct __dirstream`).
                None if r["size"].is_empty() => vec![format!("{kind} {cxx};")],
                None => vec![format!("{};", opaque(&format!("{kind} {cxx}")))],
            };
            self.guard(&format!("{kind}_{cxx}"), def);
            vec![format!("typedef {kind} {cxx} {name}{td};")]
        };
        self.guard(name, body);
    }

    /// The C++ form of a `typedefs.tsv` typedef, or an error.
    pub(super) fn cxx(&self, name: &str) -> Option<String> {
        self.t.typedefs.get(&("typedef".to_string(), name.to_string())).map(|r| r["cxx"].clone())
    }
}
