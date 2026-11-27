//! Structure, union and enumeration tags and their members, from layouts.tsv: ONE recursive member emitter for every
//! aggregate construct the rows describe (HEADERS-GEN-COMPLETE): named and C11 anonymous nested structures and unions
//! at any depth (an `<anon>` row's `size N rows K` gives its K member rows), bit-fields, fixed and flexible arrays,
//! qualified and pointer members, enumeration members, function-pointer members, structure tags described only by a
//! member's rows (`derive_layouts`), packed, over-aligned and padded layouts, `@@hidden-member`.
use super::*;

/// An aggregate's definition from its layouts.tsv rows.
pub(super) struct Agg {
    /// `struct` or `union`.
    pub(super) kw: &'static str,
    /// Attributes of the aggregate type, spelled after `kw` (` __attribute__ ((__packed__))`, `__aligned__`).
    pub(super) attrs: String,
    /// An alignment only a typedef can carry (the size is not a multiple of it, `__pthread_unwind_buf_t`): spelled
    /// after the typedef's name.
    pub(super) td_attrs: String,
    pub(super) lines: Vec<String>,
    /// The tags and typedefs the members name, emitted first ([`Gen::emit_nested`]).
    pub(super) nested: Vec<String>,
}

/// One level of members: an aggregate, or a named or anonymous aggregate nested in one.
struct Level {
    lines: Vec<String>,
    /// The end of the bytes the members cover (None: a size is unknown, as a flexible array's).
    end: Option<u64>,
    offs: BTreeSet<u64>,
    /// Declared members: a union has more than one, all at one offset (a bit-field's offset is its storage unit's).
    count: usize,
    bits: bool,
    /// A member wider than one byte: an alignment of 1 then means a packed structure.
    wide: bool,
    /// The strictest member alignment (None: one is unknown).
    align: Option<u64>,
}

impl Level {
    /// A level whose members start at `base` (table offsets are from the start of the outermost aggregate).
    fn new(base: u64) -> Self {
        Level { lines: Vec::new(), end: Some(base), offs: BTreeSet::new(), count: 0, bits: false, wide: false, align: Some(1) }
    }

    /// One member at `off`, of `size` bytes and alignment `align` (None: unknown).
    fn add(&mut self, off: u64, size: Option<u64>, align: Option<u64>) {
        self.offs.insert(off);
        self.count += 1;
        self.wide |= size.is_some_and(|n| n > 1);
        self.end = self.end.zip(size).map(|(e, n)| e.max(off + n));
        self.align = self.align.zip(align).map(|(a, b)| a.max(b));
    }

    /// Every member is one plain declaration line (no nested block, no `#if`): the level fits on one line.
    fn plain(&self) -> bool {
        self.lines.len() == self.count && !self.lines.iter().any(|l| l.starts_with('#') || l.contains('{'))
    }
}

/// `const ` and `volatile ` before a member type (`const char *` is a pointer to `const char`), and the rest.
fn quals(core: &str) -> (String, &str) {
    let mut q = String::new();
    let mut rest = core;
    while let Some((w, r)) = rest.split_once(' ').filter(|(w, _)| matches!(*w, "const" | "volatile")) {
        q.push_str(w);
        q.push(' ');
        rest = r;
    }
    (q, rest)
}

/// Whether a layouts.tsv head row marks a transparent union (`transparent_union`, then ` restrict` when the name's
/// macro form is a restrict pointer: spec/tools/abi_extract.py `tu_members`).
pub(super) fn transparent_union(head: Option<&BTreeMap<String, String>>) -> bool {
    head.is_some_and(|r| r["bitfield"].split_whitespace().next() == Some("transparent_union"))
}

/// The number after `word` in an `<anon>` row's `size N rows K`.
fn anon_number(spec: &str, word: &str) -> Option<u64> {
    let w: Vec<&str> = spec.split_whitespace().collect();
    w.iter().position(|x| *x == word).and_then(|p| w.get(p + 1)).and_then(|v| v.parse().ok())
}

impl Gen<'_> {
    /// The definition of the aggregate `key` of layouts.tsv (None when the rows cannot give one: an unresolved member
    /// type, a row out of tree order).
    pub(super) fn aggregate(&self, key: &str) -> Option<Agg> {
        let rows = self.t.layouts.get(key)?;
        let members: Vec<&BTreeMap<String, String>> = rows.iter().filter(|r| !r["member"].is_empty()).collect();
        let mut nested = Vec::new();
        let mut lv = self.level(key, &members, "", 0, &mut nested)?;
        let head = rows.iter().find(|r| r["member"].is_empty());
        let total = head.and_then(|r| r["offset_or_size"].parse::<u64>().ok());
        let align = head.and_then(|r| r["align"].parse::<u64>().ok());
        // Unnamed padding the layout rows do not list (`struct timex` ends in 44 bytes of it).
        if let (Some(end), Some(total)) = (lv.end, total) {
            if total > end && (total - end) % 4 == 0 {
                lv.lines.extend((0..(total - end) / 4).map(|_| "  int : 32;".to_string()));
            }
        }
        let kw = if lv.count > 1 && lv.offs.len() == 1 && !lv.bits { "union" } else { "struct" };
        let (mut attrs, mut td_attrs) = (String::new(), String::new());
        if transparent_union(head) {
            // A union a parameter takes as any of its members (GCC manual, transparent_union): layouts.tsv's mark.
            attrs = " __attribute__ ((__transparent_union__))".into();
        } else if lv.wide && lv.align != Some(1) && align == Some(1) {
            attrs = " __attribute__ ((__packed__))".into();
        } else if let (Some(a), Some(natural)) = (align, lv.align) {
            // Declared stricter than its members (`struct rseq`, 32): the type carries it, or (a size that is not a
            // multiple of it) only the typedef can.
            if a > natural {
                let at = format!(" __attribute__ ((__aligned__ ({a})))");
                if total.is_some_and(|t| t % a == 0) {
                    attrs = at;
                } else {
                    td_attrs = at;
                }
            }
        }
        (!lv.lines.is_empty()).then_some(Agg { kw, attrs, td_attrs, lines: lv.lines, nested })
    }

    /// The members of `rows`, one level of `key` whose member paths start with `prefix`, in table order.
    /// None when a member cannot be declared, or the level has none.
    fn level(&self, key: &str, rows: &[&BTreeMap<String, String>], prefix: &str, base: u64, nested: &mut Vec<String>) -> Option<Level> {
        let mut lv = Level::new(base);
        let mut i = 0;
        while i < rows.len() {
            let r = rows[i];
            let name = r["member"].strip_prefix(prefix)?;
            if name == "<anon>" {
                // A C11 anonymous structure or union: the next K rows are its members, named in this scope.
                let off: u64 = r["offset_or_size"].parse().ok()?;
                let k = anon_number(&r["bitfield"], "rows")? as usize;
                let kw = r["member_type"].strip_suffix(" <anon>").filter(|k| matches!(*k, "struct" | "union"))?;
                // K counts its member rows at this level (abi-gen reads it so too); the rows below each follow it.
                let dots = prefix.matches('.').count();
                let (mut end, mut seen) = (i + 1, 0);
                while let Some(q) = rows.get(end) {
                    let d = q["member"].matches('.').count();
                    if d < dots || (d == dots && seen == k) {
                        break;
                    }
                    seen += usize::from(d == dots);
                    end += 1;
                }
                if seen < k {
                    return None;
                }
                let inner = self.level(key, &rows[i + 1..end], prefix, off, nested)?;
                lv.lines.push(format!("  {kw} {{"));
                lv.lines.extend(inner.lines);
                lv.lines.push("  };".into());
                lv.add(off, anon_number(&r["bitfield"], "size").or(inner.end.map(|e| e - off)), inner.align);
                i = end;
                continue;
            }
            // The rows below a named member (`m.x`) follow it.
            let sub = format!("{}.", r["member"]);
            let n = rows[i + 1..].iter().take_while(|q| q["member"].starts_with(&sub)).count();
            self.member(&mut lv, key, r, name, &rows[i + 1..i + 1 + n], nested)?;
            i += 1 + n;
        }
        (lv.count > 0).then_some(lv)
    }

    /// One named member `name` (its row `r`, the rows below it `kids`) of a level of `key`.
    fn member(&self, lv: &mut Level, key: &str, r: &BTreeMap<String, String>, name: &str, kids: &[&BTreeMap<String, String>], nested: &mut Vec<String>) -> Option<()> {
        let mt = r["member_type"].as_str();
        let off: u64 = r["offset_or_size"].parse().ok()?;
        // A pointer to a function whose parameters the table does not record (`fn(...) *`): the program never calls it
        // through the structure, so it is declared as a pointer to `void (void)`; size and offset are those of any
        // object pointer.
        if mt.starts_with("fn(") && mt.trim_end().ends_with('*') {
            // headers/fnmembers.tsv spells the type of the members whose signature a standard or man page gives.
            let decl = self.t.fnmembers.get(&(key.to_string(), r["member"].clone())).map_or(format!("void (*{name}) (void)"), |d| d.replace('@', name));
            // `GNU-DECL ;; OTHER-DECL`: the first with _GNU_SOURCE, the second otherwise. The tags the GNU form names are
            // declared incomplete by the header's template (glob.h.in), so no dependency is pulled in for them.
            if let Some((gnu, other)) = decl.split_once(";;") {
                lv.lines.push("# ifdef __USE_GNU".into());
                lv.lines.push(format!("  {};", gnu.trim()));
                lv.lines.push("# else".into());
                lv.lines.push(format!("  {};", other.trim()));
                lv.lines.push("# endif".into());
                lv.add(off, Some(8), Some(8));
                return Some(());
            }
            // The tags and typedefs the declarator names are declared first.
            let words: Vec<&str> = decl.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect();
            for (i, w) in words.iter().enumerate() {
                let dep = match *w {
                    "struct" | "union" => words.get(i + 1).map(|t| format!("{w} {t}")),
                    _ if i > 0 && matches!(words[i - 1], "struct" | "union") => None,
                    _ if self.t.typedefs.contains_key(&("typedef".to_string(), w.to_string())) => Some(format!("typedef {w}")),
                    _ => None,
                };
                if let Some(d) = dep.filter(|d| !nested.contains(d)) {
                    nested.push(d);
                }
            }
            lv.lines.push(format!("  {decl};"));
            lv.add(off, Some(8), Some(8));
            return Some(());
        }
        // A bit-field (`bits W@B`): declared in table order with its width; its position follows from the order. Its
        // type is a scalar, or a typedef of one (`uint8_t`), spelled as the scalar.
        if let Some(w) = r["bitfield"].strip_prefix("bits ").and_then(|b| b.split('@').next()).filter(|w| w.parse::<u32>().is_ok()) {
            let ty = dwarf_scalar(mt).or_else(|| Some(self.resolve(mt)).filter(|t| scalar(t))).filter(|_| !mt.contains('*'))?;
            lv.lines.push(format!("  {ty} {name} : {w};"));
            lv.add(off, self.type_size(mt, "", ""), self.type_align(mt, ""));
            lv.bits = true;
            return Some(());
        }
        if !r["bitfield"].is_empty() {
            return None;
        }
        let (base, arr) = mt.find('[').map_or((mt, ""), |i| (&mt[..i], &mt[i..]));
        let core = base.trim_end_matches(['*', ' ']);
        let stars = &base[core.len()..];
        let n: Option<u64> = if arr.is_empty() { Some(1) } else { arr.trim_matches(['[', ']']).parse().ok() };
        // A named structure or union of the aggregate's own (`ut_tv`, `_sigev_un`): its members are the rows below it,
        // on one line when each is one plain declaration. A tag only these rows describe (no row of its own in any
        // table, so no size to define it by apart) is defined here, keeping its tag.
        // A tag headers/nested-tags.tsv puts inside this aggregate is defined here too, though it has rows of its own.
        let nested_here = self.t.nested_tags.get(core).is_some_and(|o| o == key);
        let own_tag = core.split_once(' ').is_some_and(|(k, t)| matches!(k, "struct" | "union") && t != "<anon>" && (nested_here || !self.tag_known(k, t))) && !kids.is_empty();
        if let Some(kw) = core.strip_suffix(" <anon>").or(own_tag.then_some(core)).filter(|k| stars.is_empty() && (own_tag || matches!(*k, "struct" | "union"))) {
            let inner = self.level(key, kids, &format!("{}.", r["member"]), off, nested)?;
            // The rows give a nested aggregate no size of its own. Rounded up to its members' alignment it may run
            // past the end of the outermost aggregate (`struct tftphdr`, 5 bytes: the union at offset 2 holds a
            // `char[3]` and a two-byte structure): such a member can only be packed, its size the bytes its members
            // cover.
            let total = self.t.layouts.get(key).and_then(|rs| rs.iter().find(|q| q["member"].is_empty())).and_then(|q| q["offset_or_size"].parse::<u64>().ok());
            let natural = inner.end.zip(inner.align).map(|(e, a)| (e - off).div_ceil(a) * a);
            let packed = arr.is_empty() && natural.zip(total).is_some_and(|(s, t)| off + s > t);
            let attr = if packed { " __attribute__ ((__packed__))" } else { "" };
            // The attribute follows the keyword, before a tag (`struct __attribute__ ((__packed__)) t {`).
            let kw = kw.split_once(' ').map_or(format!("{kw}{attr}"), |(k, t)| format!("{k}{attr} {t}"));
            if inner.plain() {
                let ms: Vec<&str> = inner.lines.iter().map(|l| l.trim_start()).collect();
                lv.lines.push(format!("  {kw} {{ {} }} {name}{arr};", ms.join(" ")));
            } else {
                lv.lines.push(format!("  {kw} {{"));
                lv.lines.extend(inner.lines);
                lv.lines.push(format!("  }} {name}{arr};"));
            }
            let (size, align) = if packed { (inner.end.map(|e| e - off), Some(1)) } else { (natural.zip(n).map(|(s, n)| s * n), inner.align) };
            lv.add(off, size, align);
            return Some(());
        }
        let (qual, bare) = quals(core);
        let pointer = if stars.is_empty() { String::new() } else { format!(" {}", stars.trim()) };
        // `@@hidden-member`: outside the condition of the member's structure tag, its scalar members by the given names.
        let mut hidden: Option<(String, Vec<String>)> = None;
        let ty = if let Some((kind, tag)) = bare.split_once(' ').filter(|(k, t)| stars.is_empty() && matches!(*k, "struct" | "union") && self.tag_known(k, t)) {
            if let (Some(names), Some(c)) = (self.hidden_members.get(name).filter(|_| arr.is_empty() && kind == "struct"), key.strip_prefix("struct ").and_then(|o| self.narrower_tag(("struct", o), ("struct", tag)))) {
                let direct: Vec<_> = kids.iter().filter(|q| !q["member"][r["member"].len() + 1..].contains('.')).collect();
                if direct.len() != names.len() {
                    return None;
                }
                let mut alt = Vec::new();
                for (q, n) in direct.iter().zip(names) {
                    let b = dwarf_scalar(&q["member_type"]).or_else(|| Some(self.resolve(&q["member_type"])).filter(|r| scalar(r)))?;
                    alt.push(format!("  {b} {n};"));
                }
                nested.push(format!("#if {c}\t{bare}"));
                hidden = Some((c, alt));
            } else {
                // A complete structure member (itimerspec holds two timespecs): its definition comes first.
                nested.push(bare.to_string());
            }
            format!("{qual}{bare}")
        } else if stars.is_empty() && self.t.typedefs.get(&("typedef".to_string(), bare.to_string())).is_some_and(|t| t["cxx"].ends_with(']') || self.t.layouts.contains_key(bare)) {
            // A typedef that is an array type, or an aggregate with a layout of its own: it comes first.
            nested.push(format!("typedef {bare}"));
            format!("{qual}{bare}")
        } else if stars.is_empty() && bare.starts_with("enum ") {
            // An enumeration (four bytes, no negative enumerator; `enum <anon>` too): its underlying type.
            format!("{qual}unsigned int")
        } else if bare.starts_with("struct ") || bare.starts_with("union ") {
            // A pointer to a structure: the tag may stay incomplete (`struct _IO_marker *`).
            format!("{qual}{bare}{pointer}")
        } else if self.t.typedefs.get(&("typedef".to_string(), bare.to_string())).is_some_and(|t| t["cxx"].contains('(') && (!stars.is_empty() || t["cxx"].contains("(*"))) {
            // A pointer to a function type named by a typedef (`cookie_read_function_t *`): it comes first.
            nested.push(format!("typedef {bare}"));
            format!("{qual}{bare}{pointer}")
        } else if dwarf_scalar(bare).is_none() && self.t.typedefs.contains_key(&("typedef".to_string(), bare.to_string())) && !(scalar(&self.resolve(bare)) || self.resolve(bare) == "void") {
            // A member of a typedef name that does not resolve to a scalar here (`FILE *`, `wchar_t`): the typedef is
            // emitted first and the member uses its name.
            nested.push(format!("typedef {bare}"));
            format!("{qual}{bare}{pointer}")
        } else {
            let b = dwarf_scalar(bare).or_else(|| {
                let r = self.resolve(bare);
                (scalar(&r) || r == "void").then_some(r)
            })?;
            format!("{qual}{b}{pointer}")
        };
        lv.add(off, self.type_size(bare, stars, arr), self.type_align(bare, stars));
        match hidden {
            Some((c, alt)) => {
                lv.lines.extend([format!("#if {c}"), format!("  {ty} {name}{arr};"), "#else".into()]);
                lv.lines.extend(alt);
                lv.lines.push("#endif".into());
            }
            None => lv.lines.push(format!("  {ty} {name}{arr};")),
        }
        Some(())
    }

    /// Whether `kind tag` is a structure or union tag the generator can define: rows of its own (`derive_layouts`
    /// gives a tag described by a member's rows its own), or its typedefs.tsv row (an opaque definition of its size).
    pub(super) fn tag_known(&self, kind: &str, tag: &str) -> bool {
        self.t.layouts.contains_key(&format!("{kind} {tag}")) || self.t.typedefs.get(&(kind.to_string(), tag.to_string())).is_some_and(|r| !r["size"].is_empty())
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
            } else if let Some(tag) = n.strip_prefix("union ") {
                self.tag(tag, "union");
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
        // A tag defined inside another aggregate (headers/nested-tags.tsv) comes with that aggregate's definition.
        if let Some((okind, oname)) = self.t.nested_tags.get(&key).and_then(|o| o.split_once(' ')) {
            let (okind, oname) = (okind.to_string(), oname.to_string());
            self.tag(&oname, &okind);
            return;
        }
        if kind == "enum" {
            // An enumeration: its enumerators.tsv rows (none: it cannot be declared complete).
            match self.enum_body(name) {
                Some(body) => self.guard(&format!("enum_{name}"), body),
                None => self.errors.push(format!("{}: {key} has no enumerators.tsv rows", self.header)),
            }
            return;
        }
        // `struct _IO_FILE` is described under its typedef name, FILE (layouts.tsv keys a tag by the name the program uses).
        let lkey = if self.t.layouts.contains_key(&key) {
            key.clone()
        } else {
            // A typedef of the tag's own name with no C++ spelling (`ucontext_t`) is the same class.
            self.t.typedefs.iter().find(|((k, n), r)| k == "typedef" && (r["cxx"] == name || (r["cxx"].is_empty() && n == name)) && self.t.layouts.contains_key(n.as_str())).map_or(key.clone(), |((_, n), _)| n.clone())
        };
        let size = self.t.typedefs.get(&(kind.to_string(), name.to_string())).filter(|r| !r["size"].is_empty() && kind != "enum").map(|r| (r["size"].clone(), r["align"].clone()));
        let body = match self.aggregate(&lkey) {
            Some(a) if a.td_attrs.is_empty() => {
                self.emit_nested(a.nested);
                [vec![format!("{kind}{} {name} {{", a.attrs)], a.lines, vec!["};".into()]].concat()
            }
            // No rows describe its members anywhere: an object of its size and alignment (typedefs.tsv).
            None if !self.t.layouts.contains_key(&lkey) && size.is_some() => {
                let (s, al) = size.unwrap_or_default();
                vec![format!("{key} {{ char __niji_opaque[{s}] __attribute__ ((__aligned__ ({al}))); }};")]
            }
            _ => {
                self.errors.push(format!("{}: {key} has no usable layouts.tsv rows", self.header));
                return;
            }
        };
        self.guard(&format!("{kind}_{name}"), body);
    }
}
