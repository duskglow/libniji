//! Macro values: the C literal of each `macros.tsv` row, typed (an integer constant keeps its type as a suffix).
use super::*;

/// The C literal of a `macros.tsv` value of the given type.
pub(crate) fn literal(ty: &str, value: &str) -> Result<String, String> {
    if ty.starts_with("char[") {
        return Ok(value.to_string());
    }
    let suffix = match ty {
        "int" => "",
        "long" => "L",
        "long long" => "LL",
        "unsigned int" => "U",
        "unsigned long" => "UL",
        "unsigned long long" => "ULL",
        _ => return Err(format!("no literal form for type {ty:?}")),
    };
    if let Some(mag) = value.strip_prefix('-') {
        let mag: u128 = mag.parse().map_err(|_| format!("bad value {value:?}"))?;
        let min = match ty {
            "int" => 1u128 << 31,
            _ => 1u128 << 63,
        };
        // The most negative value has no literal: write it as (-(max) - 1).
        return Ok(if mag == min { format!("(-{}{suffix}-1)", mag - 1) } else { format!("(-{mag}{suffix})") });
    }
    value.parse::<u128>().map_err(|_| format!("bad value {value:?}"))?;
    Ok(format!("{value}{suffix}"))
}

impl Gen<'_> {
    pub(super) fn value(&mut self, name: &str) -> String {
        match self.t.macros.get(name).map(|(ty, v)| literal(ty, v)) {
            Some(Ok(s)) => s,
            Some(Err(e)) => {
                self.errors.push(format!("{}: {name}: {e}", self.header));
                String::new()
            }
            None => {
                self.errors.push(format!("{}: {name} has no macros.tsv value", self.header));
                String::new()
            }
        }
    }
}
