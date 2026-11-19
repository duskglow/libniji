//! Structure, union and enumeration tags and their members, from layouts.tsv: nested and anonymous aggregates,
//! bit-fields, arrays, packed and padded layouts, function-pointer members, hidden members.
use super::*;

impl Gen<'_> {
    /// The members of an aggregate from layouts.tsv (None when it must be opaque: bit-field or unresolved member
    /// types) and the structure tags and function typedefs its members need first.
    pub(super) fn members_nested(&self, key: &str) -> Option<(String, Vec<String>, Vec<String>)> {
        let mut nested: Vec<String> = Vec::new();
        let rows = self.t.layouts.get(key)?;
        let mut out = Vec::new();
        let mut offs = BTreeSet::new();
        // Members that are complete structures: the rows `member.sub` below them describe that structure again.
        let whole: BTreeSet<&str> = rows
            .iter()
            .filter(|r| self.t.layouts.contains_key(&r["member_type"]))
            .map(|r| r["member"].as_str())
            .collect();
        // Members that are unnamed structures or unions (`ut_tv`): their members are the rows `member.sub` below.
        // A structure tag with no rows of its own (utmpx's `struct __exit_status`) is described the same way, keeping its tag.
        let is_agg = |r: &BTreeMap<String, String>| {
            let mt = r["member_type"].as_str();
            key.starts_with("struct ") && !r["member"].contains('.') && r["member"] != "<anon>" && (mt.ends_with(" <anon>") || (mt.starts_with("struct ") && !self.t.layouts.contains_key(mt) && rows.iter().any(|q| q["member"].starts_with(&format!("{}.", r["member"])))))
        };
        let anon: BTreeSet<&str> = rows.iter().filter(|r| is_agg(r)).map(|r| r["member"].as_str()).collect();
        let mut end = Some(0u64);
        let mut wide = false;
        // An anonymous union (`<anon>`, SL-HEADERS-S09): the rows after it at its offset are its members.
        let mut open_anon: Option<String> = None;
        for r in rows.iter().filter(|r| !r["member"].is_empty()) {
            if let Some(kw) = is_agg(r).then_some(r["member_type"].strip_suffix(" <anon>").unwrap_or(&r["member_type"])) {
                // One level of an unnamed aggregate whose members are scalars or arrays of them.
                let prefix = format!("{}.", r["member"]);
                let mut inner = Vec::new();
                for q in rows.iter().filter(|q| q["member"].strip_prefix(prefix.as_str()).is_some_and(|m| !m.contains('.'))) {
                    let mt = &q["member_type"];
                    let (base, arr) = mt.find('[').map_or((mt.as_str(), ""), |i| (&mt[..i], &mt[i..]));
                    // A pointer member (`void *tempptr`) is declared with its pointee spelled as a scalar or `void`.
                    let core = base.trim_end_matches(['*', ' ']);
                    let stars = if base.len() > core.len() { " *" } else { "" };
                    let b = if core == "void" && !stars.is_empty() { "void".to_string() } else { dwarf_scalar(core).or_else(|| Some(self.resolve(core)).filter(|r| scalar(r)))? };
                    inner.push(format!("{b}{stars} {}{arr};", &q["member"][prefix.len()..]));
                }
                let off: u64 = r["offset_or_size"].parse().ok()?;
                offs.insert(r["offset_or_size"].clone());
                end = end.zip(rows.iter().filter(|q| q["member"].starts_with(prefix.as_str())).filter_map(|q| Some(q["offset_or_size"].parse::<u64>().ok()? + self.type_size(q["member_type"].split('[').next()?, "", q["member_type"].find('[').map_or("", |i| &q["member_type"][i..]))?)).max()).map(|(e, n)| e.max(n)).or(Some(off));
                out.push(format!("  {kw} {{ {} }} {};", inner.join(" "), r["member"]));
                continue;
            }
            if r["member"].split_once('.').is_some_and(|(parent, _)| anon.contains(parent)) {
                continue;
            }
            if r["member"].split_once('.').is_some_and(|(parent, _)| whole.contains(parent)) {
                continue;
            }
            if open_anon.as_ref().is_some_and(|o| *o != r["offset_or_size"] || r["member"] == "<anon>") {
                out.push("  };".into());
                open_anon = None;
            }
            if r["member"] == "<anon>" && r["member_type"] == "union <anon>" {
                out.push("  union {".into());
                open_anon = Some(r["offset_or_size"].clone());
                continue;
            }
            if r["member"].contains('.') {
                return None;
            }
            let mt = &r["member_type"];
            let off0: u64 = r["offset_or_size"].parse().ok()?;
            // A pointer to a function whose parameters the table does not record (`fn(...) *`): the program never
            // calls it through the structure, so it is declared as a pointer to `void (void)`; size and offset are
            // those of any object pointer.
            if mt.starts_with("fn(") && mt.trim_end().ends_with('*') {
                end = end.map(|e| e.max(off0 + 8));
                offs.insert(r["offset_or_size"].clone());
                out.push(format!("  void (*{}) (void);", r["member"]));
                continue;
            }
            // A bit-field (`bits W@B`): declared in table order with its width; its position follows from the order.
            if let Some(w) = r["bitfield"].strip_prefix("bits ").and_then(|b| b.split('@').next()).filter(|w| w.parse::<u32>().is_ok()) {
                let ty = dwarf_scalar(mt).filter(|_| !mt.contains('*'))?;
                offs.insert(r["offset_or_size"].clone());
                end = end.zip(self.type_size(mt, "", "")).map(|(e, n)| e.max(off0 + n));
                out.push(format!("  {ty} {} : {w};", r["member"]));
                continue;
            }
            if !r["bitfield"].is_empty() {
                return None;
            }
            let (base, arr) = match mt.find('[') {
                Some(i) => (&mt[..i], &mt[i..]),
                None => (mt.as_str(), ""),
            };
            let core = base.trim_end_matches(['*', ' ']);
            let stars = &base[core.len()..];
            // `@@hidden-member`: outside the condition of the member's structure tag, its scalar members by the given names.
            let mut hidden: Option<(String, Vec<String>)> = None;
            let base = if let Some(tag) = core.strip_prefix("struct ").filter(|_| stars.is_empty() && self.t.layouts.contains_key(core)) {
                if let (Some(names), Some(c)) = (self.hidden_members.get(r["member"].as_str()).filter(|_| arr.is_empty()), key.strip_prefix("struct ").and_then(|o| self.narrower_tag(o, tag))) {
                    let prefix = format!("{}.", r["member"]);
                    let subs: Vec<_> = rows.iter().filter(|q| q["member"].strip_prefix(prefix.as_str()).is_some_and(|m| !m.contains('.'))).collect();
                    if subs.len() != names.len() {
                        return None;
                    }
                    let mut alt = Vec::new();
                    for (q, n) in subs.iter().zip(names) {
                        let b = dwarf_scalar(&q["member_type"]).or_else(|| Some(self.resolve(&q["member_type"])).filter(|r| scalar(r)))?;
                        alt.push(format!("  {b} {n};"));
                    }
                    nested.push(format!("#if {c}\t{core}"));
                    hidden = Some((c, alt));
                } else {
                    // A complete structure member (itimerspec holds two timespecs): its definition comes first.
                    nested.push(core.to_string());
                }
                format!("struct {tag}")
            } else if stars.is_empty() && self.t.typedefs.get(&("typedef".to_string(), core.to_string())).is_some_and(|t| t["cxx"].ends_with(']') || self.t.layouts.contains_key(core)) {
                // A typedef that is an array type, or an aggregate with a layout of its own: it comes first.
                nested.push(format!("typedef {core}"));
                core.to_string()
            } else if stars.is_empty() && core.starts_with("enum ") && self.t.typedefs.contains_key(&("enum".to_string(), core["enum ".len()..].to_string())) {
                // An enumeration (four bytes, no negative enumerator): its underlying type.
                "unsigned int".to_string()
            } else if core.trim_start_matches("const ").starts_with("struct ") && !stars.is_empty() {
                // A pointer to a structure: the tag may stay incomplete (`struct _IO_marker *`).
                format!("{core} {}", stars.trim())
            } else if self.t.typedefs.get(&("typedef".to_string(), core.to_string())).is_some_and(|t| t["cxx"].contains('(') && (!stars.is_empty() || t["cxx"].contains("(*"))) {
                // A pointer to a function type named by a typedef (`cookie_read_function_t *`): it comes first.
                nested.push(format!("typedef {core}"));
                format!("{core} {}", stars.trim())
            } else if dwarf_scalar(core).is_none() && self.t.typedefs.contains_key(&("typedef".to_string(), core.to_string())) && !(scalar(&self.resolve(core)) || self.resolve(core) == "void") {
                // A member of a typedef name that does not resolve to a scalar here (`FILE *`, `wchar_t`): the typedef
                // is emitted first and the member uses its name.
                nested.push(format!("typedef {core}"));
                format!("{core}{}", if stars.is_empty() { String::new() } else { format!(" {}", stars.trim()) })
            } else {
                let b = dwarf_scalar(core).or_else(|| {
                    let r = self.resolve(core);
                    (scalar(&r) || r == "void").then_some(r)
                })?;
                format!("{b}{}", if stars.is_empty() { String::new() } else { format!(" {}", stars.trim()) })
            };
            let off: u64 = r["offset_or_size"].parse().ok()?;
            let size = self.type_size(core, stars, arr);
            wide |= size.is_some_and(|n| n > 1);
            end = size.zip(end).map(|(n, e)| e.max(off + n));
            offs.insert(r["offset_or_size"].clone());
            match hidden {
                Some((c, alt)) => {
                    out.extend([format!("#if {c}"), format!("  {base} {}{arr};", r["member"]), "#else".into()]);
                    out.extend(alt);
                    out.push("#endif".into());
                }
                None => out.push(format!("  {base} {}{arr};", r["member"])),
            }
        }
        // Unnamed padding the layout rows do not list (`struct timex` ends in 44 bytes of it).
        if let (Some(end), Some(total)) = (end, rows.iter().find(|r| r["member"].is_empty()).and_then(|r| r["offset_or_size"].parse::<u64>().ok())) {
            if total > end && (total - end) % 4 == 0 {
                out.extend((0..(total - end) / 4).map(|_| "  int : 32;".to_string()));
            }
        }
        if open_anon.is_some() {
            out.push("  };".into());
        }
        let packed = wide && rows.iter().find(|r| r["member"].is_empty()).is_some_and(|r| r["align"] == "1");
        let kw = if out.len() > 1 && offs.len() == 1 {
            "union"
        } else if packed {
            "struct __attribute__ ((__packed__))"
        } else {
            "struct"
        };
        (!out.is_empty()).then(|| (kw.to_string(), out, nested))
    }

    /// Emit what a structure's members need first: the structures and function typedefs they name.
    pub(super) fn emit_nested(&mut self, nested: Vec<String>) {
        for n in nested {
            // `#if <condition>\tstruct TAG` (`@@hidden-member`): the tag is defined only under its own condition.
            if let Some((c, tag)) = n.strip_prefix("#if ").and_then(|r| r.split_once('\t')).and_then(|(c, t)| Some((c.to_string(), t.strip_prefix("struct ")?.to_string()))) {
                self.out.push(format!("# if {c}"));
                self.tag(&tag, "struct");
                self.out.push("# endif".into());
            } else if let Some(tag) = n.strip_prefix("struct ") {
                self.tag(tag, "struct");
            } else if let Some(t) = n.strip_prefix("typedef ") {
                self.typedef(t);
            }
        }
    }

    /// Whether `name` is the structure tag a typedef with layouts.tsv rows is spelled through (`_IO_FILE` for `FILE`).
    pub(super) fn tag_of_typedef(&self, name: &str) -> bool {
        self.t.typedefs.iter().any(|((k, n), r)| k == "typedef" && r["cxx"] == name && self.t.layouts.contains_key(n.as_str()))
    }

    pub(super) fn tag(&mut self, name: &str, kind: &str) {
        let key = format!("{kind} {name}");
        // `struct _IO_FILE` is described under its typedef name, FILE (layouts.tsv keys a tag by the name the program uses).
        let lkey = if self.t.layouts.contains_key(&key) {
            key.clone()
        } else {
            self.t.typedefs.iter().find(|((k, n), r)| k == "typedef" && r["cxx"] == name && self.t.layouts.contains_key(n.as_str())).map_or(key.clone(), |((_, n), _)| n.clone())
        };
        let body = match self.members_nested(&lkey) {
            Some((kw, m, nested)) => {
                self.emit_nested(nested);
                let head = if kw.contains("packed") { format!("{kind} __attribute__ ((__packed__)) {name} {{") } else { format!("{key} {{") };
                [vec![head], m, vec!["};".into()]].concat()
            }
            None => {
                self.errors.push(format!("{}: {key} has no usable layouts.tsv rows", self.header));
                return;
            }
        };
        self.guard(&format!("{kind}_{name}"), body);
    }
}
