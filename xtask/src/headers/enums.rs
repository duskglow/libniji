//! Enumerations: a named enumeration from its enumerators.tsv rows (HEADERS-TABLES-COMPLETE), through the tag emitter,
//! an enumerator of it, or `@@enum TAG`.
use super::*;

impl Gen<'_> {
    /// The tag (`__pid_type`) of the enumeration `name` is an enumerator of, when the tables declare that enumeration
    /// as a tag (a typedefs.tsv `enum` row): its enumerators are then emitted only as its members.
    pub(super) fn declared_enum(&self, name: &str) -> Option<String> {
        let tag = self.t.enum_of.get(name)?.strip_prefix("enum ")?;
        self.t.typedefs.contains_key(&("enum".to_string(), tag.to_string())).then(|| tag.to_string())
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
        let body: Vec<String> = names.iter().flat_map(|n| [format!("# ifndef {n}"), format!("#  define {n} {n}"), "# endif".into()]).collect();
        self.out.extend(body);
    }
}
