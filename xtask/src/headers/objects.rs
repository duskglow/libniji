//! Data objects: the `extern` declaration of each typedefs.tsv `object` row.
use super::*;

impl Gen<'_> {
    pub(super) fn object(&mut self, name: &str) {
        let Some(r) = self.t.typedefs.get(&("object".to_string(), name.to_string())) else {
            self.errors.push(format!("{}: object {name} not in typedefs.tsv", self.header));
            return;
        };
        let ty = r["cxx"].clone();
        let line = if let Some(open) = ty.find("(*)") {
            // A pointer to a function (`void (*)()`): the name goes inside the parentheses. An empty list is `(void)` in C.
            let (ret, rest) = ty.split_at(open);
            let args = self.type_words(&rest[3..]);
            let args = if args == "()" { "(void)".to_string() } else { args };
            let ret = self.type_words(ret);
            format!("extern {} (*{name}){args};", ret.trim_end())
        } else {
            // The type spelled for C: a tag gets its keyword (`in6_addr const`, `_IO_FILE*`), a typedef comes first.
            let i = ty.find('[').unwrap_or(ty.len());
            format!("extern {} {name}{};", self.type_words(&ty[..i]), &ty[i..])
        };
        self.guard(name, vec![line]);
    }
}
