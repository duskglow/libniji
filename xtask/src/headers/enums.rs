//! Enumerations: a named enumeration from its enumerators.tsv rows (HEADERS-TABLES-COMPLETE), through the tag emitter,
//! an enumerator of it, or `@@enum TAG`.
use super::*;

impl Gen<'_> {
    /// The tag (`__pid_type`) of the enumeration `name` is an enumerator of (an enumerators.tsv `enum TAG` row): its
    /// enumerators are then emitted only as its members, so they are one type in C++ as the reference's (`EPOLLIN`
    /// and `EPOLLOUT` are both `EPOLL_EVENTS`), whether or not typedefs.tsv also has the tag (HEADERS-CXX-ENUM-TYPES).
    pub(super) fn declared_enum(&self, name: &str) -> Option<String> {
        self.t.enum_of.get(name)?.strip_prefix("enum ").map(String::from)
    }

    /// The typedef (`idtype_t`) naming the unnamed enumeration `name` is an enumerator of: an enumerators.tsv tag
    /// `typedef NAME` (ruling #7643) whose typedefs.tsv `typedef` row exists. Its enumerators are emitted only as
    /// members of that typedef's definition.
    pub(super) fn typedef_enum(&self, name: &str) -> Option<String> {
        let td = self.t.enum_of.get(name)?.strip_prefix("typedef ")?;
        self.t.typedefs.contains_key(&("typedef".to_string(), td.to_string())).then(|| td.to_string())
    }

    /// `typedef enum { ... } NAME;` for an enumerators.tsv tag `typedef NAME`: an enumeration without a tag, which C
    /// and C++ name only through the typedef (C 6.7.8; C++ names it for linkage). Its enumerators are marked emitted.
    pub(super) fn typedef_enum_body(&mut self, name: &str) -> Option<Vec<String>> {
        let members = self.t.enums.get(&format!("typedef {name}"))?;
        let mut body = vec!["typedef enum {".to_string()];
        body.extend(members.iter().map(|(n, v)| format!("  {n} = {v},")));
        body.push(format!("}} {name};"));
        for (n, _) in members {
            self.emitted.insert(format!("enumerator {n}"));
        }
        Some(body)
    }

    /// `enum TAG { ... };` from enumerators.tsv (values as the reference's DWARF gives them). Its enumerators are
    /// marked emitted in this header; elsewhere an enumerator of it is never declared alone ([`Self::declared_enum`]
    /// routes it to this definition, under the tag's guard).
    pub(super) fn enum_body(&mut self, tag: &str) -> Option<Vec<String>> {
        let members = self.t.enums.get(&format!("enum {tag}"))?;
        let mut body = vec![format!("enum {tag} {{")];
        body.extend(members.iter().map(|(n, v)| format!("  {n} = {v},")));
        body.push("};".into());
        for (n, _) in members {
            self.emitted.insert(format!("enumerator {n}"));
        }
        Some(body)
    }

    /// The tag (`anon _SC_ARG_MAX`) of the enumeration without a name `name` is an enumerator of: enumerators.tsv
    /// rows of the reference's enumerations that neither a tag nor a typedef names (HEADERS-CXX-ENUM-TYPES). Its
    /// enumerators are one type in C++ (`decltype (_SC_ARG_MAX)` is `decltype (_SC_PAGESIZE)`), as the reference's.
    pub(super) fn anon_enum(&self, name: &str) -> Option<String> {
        self.t.enum_of.get(name).filter(|t| t.starts_with("anon ")).cloned()
    }

    /// The members of the enumeration without a name `tag` this header shows, with the condition of each: its own
    /// enumerator row, or (a reserved enumerator `__niji_NAME` that only the macro NAME names) that macro's row.
    /// `all`: every member, unconditionally (an enumerator emitted outside any header: the rows test).
    fn anon_members(&mut self, tag: &str, all: bool) -> Vec<(String, String, String)> {
        let members = self.t.enums.get(tag).cloned().unwrap_or_default();
        if all {
            return members.into_iter().map(|(n, v)| (n, v, "1".to_string())).collect();
        }
        let mut out = Vec::new();
        for (n, v) in members {
            let c = if self.t.vis_row(&self.header, &n, "enumerator").is_some() {
                self.cond(&n, Some("enumerator"))
            } else if let Some(m) = n.strip_prefix("__niji_").filter(|m| self.t.vis_row(&self.header, m, "macro").is_some()) {
                let m = m.to_string();
                self.cond(&m, Some("macro"))
            } else {
                continue;
            };
            if c != "0" {
                out.push((n, v, c));
            }
        }
        out
    }

    /// The condition under which this header declares the enumeration without a name `tag`: that of any member it
    /// shows.
    pub(super) fn anon_cond(&mut self, tag: &str) -> String {
        let mut cs: Vec<String> = self.anon_members(tag, false).into_iter().map(|(_, _, c)| c).collect();
        cs.sort();
        cs.dedup();
        match cs.as_slice() {
            [] => "0".into(),
            [c] => c.clone(),
            _ if cs.iter().any(|c| c == "1") => "1".into(),
            _ => cs.iter().map(|c| format!("({c})")).collect::<Vec<_>>().join(" || "),
        }
    }

    /// `enum { ... };` of the enumeration without a name `tag`, once per translation unit (whichever header comes
    /// first declares it: the reference's enumerations have the same members in every header that shows them). A
    /// member that a macro of the same name already provides (a kernel header that came first) is left out, as a
    /// single enumerator is; a member this header shows under a narrower condition than the enumeration's is under it.
    /// The enumeration is declared only when one of those members is left (`enum { };` is invalid: `<linux/in6.h>`
    /// before `<netinet/in.h>` defines every `IPPROTO_HOPOPTS` ... `IPPROTO_MH` as a macro). `all`: as `anon_members`.
    pub(super) fn anon_body(&mut self, tag: &str, cond: &str, all: bool) {
        let members = self.anon_members(tag, all);
        let narrow = |c: &str| c != cond && c != "1";
        let terms: Vec<String> = members.iter().map(|(n, _, c)| if narrow(c) { format!("(({c}) && !defined {n})") } else { format!("!defined {n}") }).collect();
        let mut test = String::new();
        for (k, chunk) in terms.chunks(6).enumerate() {
            if k > 0 {
                test.push_str(" \\\n    || ");
            }
            test.push_str(&chunk.join(" || "));
        }
        let mut body = vec![format!("#  if {test}"), "enum {".to_string()];
        for (n, v, c) in members {
            let under = narrow(&c);
            if under {
                body.push(format!("#  if {c}"));
            }
            body.extend([format!("#  ifndef {n}"), format!("  {n} = {v},"), "#  endif".into()]);
            if under {
                body.push("#  endif".into());
            }
            self.emitted.insert(format!("enumerator {n}"));
        }
        body.extend(["};".into(), "#  endif".into()]);
        let key = format!("anon_{}", &tag["anon ".len()..]);
        self.guard(&key, body);
    }

    /// The enumerator the macro `name` names in this header (enum-macros.tsv), when this header declares that
    /// enumerator wherever it defines the macro: the macro then has the enumeration's type in C++, and in `#if` the
    /// value 0 of an identifier, as the reference's.
    pub(super) fn enum_macro(&mut self, name: &str) -> Option<String> {
        let (e, not_in) = self.t.enum_macros.get(name)?.clone();
        if not_in.contains(&self.header) {
            return None;
        }
        let mrow = self.t.vis_row(&self.header, name, "macro")?.clone();
        let mmask = mask_of(&self.t.lv, &mrow["configs"]).ok()?;
        // An enumerator without a row of its own here (`PTRACE_ATTACH` for `PT_ATTACH`, a reserved `__RLIMIT_NLIMITS`)
        // is declared where this header declares its whole enumeration (`@@enum`, or a member's row).
        if self.t.vis_row(&self.header, &e, "enumerator").is_none() && !e.starts_with("__niji_") {
            let whole = self.t.enum_of.get(&e).is_some_and(|t| self.emitted.contains(t));
            return whole.then_some(e);
        }
        let erow = if let Some(m) = e.strip_prefix("__niji_") {
            (m == name).then(|| mrow.clone())
        } else {
            self.t.vis_row(&self.header, &e, "enumerator").cloned()
        }?;
        let emask = mask_of(&self.t.lv, &erow["configs"]).ok()?;
        (mmask & !emask == 0).then_some(e)
    }

    /// The macro name `name` is defined as in this header (macro-aliases.tsv: `AF_INET` as `PF_INET`), with None when
    /// this header shows that name, as a macro or an enumerator, in every configuration in which it shows `name`.
    /// When it shows the alias in only some of them (`SO_RCVTIMEO` as `SO_RCVTIMEO_OLD`, which strict ISO C does not
    /// show), the second field is the condition that selects those, and the macro keeps its value elsewhere (the alias
    /// would expand to an undefined name). None when no condition over the levels separates them.
    pub(super) fn macro_alias(&mut self, name: &str) -> Option<(String, Option<String>)> {
        let a = self.t.macro_aliases.get(&(self.header.clone(), name.to_string()))?.clone();
        let mrow = self.t.vis_row(&self.header, name, "macro")?;
        let mmask = mask_of(&self.t.lv, &mrow["configs"]).ok()?;
        let arow = self.t.vis_row(&self.header, &a, "macro").or_else(|| self.t.vis_row(&self.header, &a, "enumerator"))?;
        let amask = mask_of(&self.t.lv, &arow["configs"]).ok()?;
        if mmask & !amask == 0 {
            return Some((a, None));
        }
        let care = mmask & !self.dont_care(name, "macro");
        if care & amask == 0 {
            return None;
        }
        let fortify = self.fortify_mask();
        let c = condition(&self.t.lv, amask & care, care).or_else(|| condition(&self.t.lv, amask & care & !fortify, care & !fortify))?;
        Some((a, Some(c)))
    }

    /// `@@enum TAG`: the enumeration `TAG` from enumerators.tsv, and each of its enumerators this header lists in
    /// visibility.tsv also a self-naming macro, so `#ifdef NAME` holds. The enumerators are then not emitted again: a
    /// prototype takes the enumeration (`ptrace (enum __ptrace_request, ...)`).
    pub(super) fn enumeration(&mut self, args: &[&str]) {
        let [tag] = args else {
            self.errors.push(format!("{}: @@enum takes one tag: {}", self.header, args.join(" ")));
            return;
        };
        let key = format!("enum {tag}");
        let Some(members) = self.t.enums.get(&key) else {
            self.errors.push(format!("{}: @@enum {tag}: no enumerators.tsv rows", self.header));
            return;
        };
        let mut names: Vec<String> = members.iter().map(|(n, _)| n.clone()).filter(|n| self.t.vis_row(&self.header, n, "enumerator").is_some()).collect();
        names.sort();
        self.tag(tag, "enum");
        self.emitted.insert(key.clone());
        let body: Vec<String> = names.iter().flat_map(|n| [format!("# ifndef {n}"), format!("#  define {n} {n}"), "# endif".into()]).collect();
        self.out.extend(body);
    }
}
