//! The defaults localedef writes for a category a source does not define (spec §31.8, #7534;
//! D-31.24). XBD 7.3 leaves a category whose definition is missing unspecified, so these are the
//! values the reference writes for an undefined category (probe-settled, black box:
//! `tests/reference.rs` `undefined_categories_get_the_references_defaults` compares every item):
//! `LC_CTYPE`'s `print` and `graph` only the letters, digits (and space), no `cntrl` or `punct`;
//! the "not available" monetary numbers 255; empty alternative month names; country codes of
//! spaces. The text is a locale source, compiled by the same writers as any source, for the run's
//! charmap; the few items a source cannot state (empty wide `LC_TIME` names, no wide input digits,
//! `mon_grouping` and CRNCYSTR) the writers give when told they compile these defaults.

/// The defaults, one section per category.
pub const SOURCE: &str = r#"LC_CTYPE
upper <U0041>..<U005A>
lower <U0061>..<U007A>
alpha <U0041>..<U005A>;<U0061>..<U007A>
digit <U0030>..<U0039>
space <U0009>..<U000D>;<U0020>
graph <U0030>..<U0039>;<U0041>..<U005A>;<U0061>..<U007A>
print <U0020>;<U0030>..<U0039>;<U0041>..<U005A>;<U0061>..<U007A>
xdigit <U0030>..<U0039>;<U0041>..<U0046>;<U0061>..<U0066>
blank <U0009>;<U0020>
toupper (<U0061>,<U0041>);(<U0062>,<U0042>);(<U0063>,<U0043>);(<U0064>,<U0044>);(<U0065>,<U0045>);(<U0066>,<U0046>);(<U0067>,<U0047>);(<U0068>,<U0048>);(<U0069>,<U0049>);(<U006A>,<U004A>);(<U006B>,<U004B>);(<U006C>,<U004C>);(<U006D>,<U004D>);(<U006E>,<U004E>);(<U006F>,<U004F>);(<U0070>,<U0050>);(<U0071>,<U0051>);(<U0072>,<U0052>);(<U0073>,<U0053>);(<U0074>,<U0054>);(<U0075>,<U0055>);(<U0076>,<U0056>);(<U0077>,<U0057>);(<U0078>,<U0058>);(<U0079>,<U0059>);(<U007A>,<U005A>)
tolower (<U0041>,<U0061>);(<U0042>,<U0062>);(<U0043>,<U0063>);(<U0044>,<U0064>);(<U0045>,<U0065>);(<U0046>,<U0066>);(<U0047>,<U0067>);(<U0048>,<U0068>);(<U0049>,<U0069>);(<U004A>,<U006A>);(<U004B>,<U006B>);(<U004C>,<U006C>);(<U004D>,<U006D>);(<U004E>,<U006E>);(<U004F>,<U006F>);(<U0050>,<U0070>);(<U0051>,<U0071>);(<U0052>,<U0072>);(<U0053>,<U0073>);(<U0054>,<U0074>);(<U0055>,<U0075>);(<U0056>,<U0076>);(<U0057>,<U0077>);(<U0058>,<U0078>);(<U0059>,<U0079>);(<U005A>,<U007A>)
END LC_CTYPE
LC_NUMERIC
decimal_point "."
thousands_sep ""
END LC_NUMERIC
LC_TIME
abday "Sun";"Mon";"Tue";"Wed";"Thu";"Fri";"Sat"
day "Sunday";"Monday";"Tuesday";"Wednesday";"Thursday";"Friday";"Saturday"
abmon "Jan";"Feb";"Mar";"Apr";"May";"Jun";"Jul";"Aug";"Sep";"Oct";"Nov";"Dec"
mon "January";"February";"March";"April";"May";"June";"July";"August";"September";"October";"November";"December"
am_pm "AM";"PM"
d_t_fmt "%a %b %e %H:%M:%S %Y"
d_fmt "%m/%d/%y"
t_fmt "%H:%M:%S"
t_fmt_ampm "%I:%M:%S %p"
date_fmt "%a %b %e %H:%M:%S %Z %Y"
week 7;19971130;7
alt_mon "";"";"";"";"";"";"";"";"";"";"";""
ab_alt_mon "";"";"";"";"";"";"";"";"";"";"";""
END LC_TIME
LC_COLLATE
END LC_COLLATE
LC_MONETARY
int_curr_symbol ""
currency_symbol ""
mon_decimal_point "."
mon_thousands_sep ""
positive_sign ""
negative_sign ""
int_frac_digits 255
frac_digits 255
p_cs_precedes 255
p_sep_by_space 255
n_cs_precedes 255
n_sep_by_space 255
p_sign_posn 255
n_sign_posn 255
END LC_MONETARY
LC_MESSAGES
yesexpr "^[yY]"
noexpr "^[nN]"
END LC_MESSAGES
LC_PAPER
height 297
width 210
END LC_PAPER
LC_NAME
name_fmt "%p%t%g%t%m%t%f"
END LC_NAME
LC_ADDRESS
postal_fmt "%a%N%f%N%d%N%b%N%s %h %e %r%N%C-%z %T%N%c%N"
country_ab2 "  "
country_ab3 "   "
END LC_ADDRESS
LC_TELEPHONE
tel_int_fmt "+%c %a%t%l"
END LC_TELEPHONE
LC_MEASUREMENT
measurement 1
END LC_MEASUREMENT
LC_IDENTIFICATION
END LC_IDENTIFICATION
"#;
