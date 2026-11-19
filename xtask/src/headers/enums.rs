//! Enumerations: `@@enum TAG PREFIX...`, a named enumeration of a header's enumerators.
use super::*;

impl Gen<'_> {
    /// `@@enum TAG PREFIX...`: `enum TAG { ... }` of the enumerators of this header named with one of the prefixes
    /// (values from macros.tsv), each also a self-naming macro so `#ifdef NAME` holds. The enumerators are then not
    /// emitted again: a prototype takes the enumeration (`ptrace (enum __ptrace_request, ...)`).
    pub(super) fn enumeration(&mut self, args: &[&str]) {
        let Some((tag, prefixes)) = args.split_first() else { return };
        let mut names: Vec<String> = self.t.vis.iter().filter(|r| r["header"] == self.header && r["kind"] == "enumerator" && prefixes.iter().any(|p| r["name"].starts_with(p))).map(|r| r["name"].clone()).collect();
        names.sort();
        names.dedup();
        if names.is_empty() {
            self.errors.push(format!("{}: @@enum {tag}: no enumerator has the prefixes", self.header));
            return;
        }
        let mut body = vec![format!("enum {tag} {{")];
        for n in &names {
            let v = self.value(n);
            body.push(format!("  {n} = {v},"));
            self.emitted.insert(format!("enumerator {n}"));
        }
        body.push("};".into());
        for n in &names {
            body.extend([format!("# ifndef {n}"), format!("#  define {n} {n}"), "# endif".into()]);
        }
        self.guard(&format!("enum_{tag}"), body);
    }
}
