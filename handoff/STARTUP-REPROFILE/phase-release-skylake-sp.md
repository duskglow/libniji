| ld.so self-relocation (relocate_self + publish) | 17,869 (5.4%) | 17,869 (3.6%) | 17,869 (4.5%) |
| symbolic relocations (reloc::one: decode, lookup, version origin, store) | 70,842 (21.3%) | 107,895 (21.9%) | 120,625 (30.7%) |
| RELATIVE relocations and table walk (pass1 less one) | 47,605 (14.3%) | 63,348 (12.9%) | 52,476 (13.3%) |
| per-object tables (Object::with_style = Object::new) | 83,822 (25.3%) | 107,505 (21.8%) | 85,956 (21.9%) |
|   of which GnuHash::symbol_count | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| whole-file image view and span checks (image_view) | 40,213 (12.1%) | 43,237 (8.8%) | 42,727 (10.9%) |
| version requirements (check_versions) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| search and open (search, open_candidate) | 20,966 (6.3%) | 20,934 (4.2%) | 20,966 (5.3%) |
| mapping (map_planned) | 2,136 (0.6%) | 9,792 (2.0%) | 2,136 (0.5%) |
| RELRO (protect_relro) | 3,284 (1.0%) | 8,168 (1.7%) | 3,284 (0.8%) |
| debugger rendezvous, publish dynamic | 8,777 (2.6%) | 16,207 (3.3%) | 9,001 (2.3%) |
| init hooks and running initialisers | 3,571 (1.1%) | 6,076 (1.2%) | 3,210 (0.8%) |
| libc start, main, exit (_start) | 1,807 (0.5%) | 2,162 (0.4%) | 0 (0.0%) |
| rest of the loader (link and entry code, env, TLS, loader exports, helpers) | 30,944 (9.3%) | 89,762 (18.2%) | 35,052 (8.9%) |
| total (PT samples) | 331,836 | 492,955 | 393,302 |
| cross: <niji_ld::elf::Elf>::read_phdr incl | 47,847 | 86,961 | 48,216 |
| cross: <niji_ld::elf::Elf>::image_span_kind incl | 29,487 | 32,475 | 31,971 |
| cross: <niji_ld::elf::Dynamic>::once incl | 22,004 | 42,223 | 24,673 |
| cross: <niji_ld::elf::Verdefs as core::iter::traits::iterator::Iterator>::next incl | 21,253 | 21,253 | 21,253 |
| cross: <niji_ld::elf::Dynamic>::hashed_string incl | 0 | 0 | 0 |
| cross: niji_ld::symbols::resolve incl | 32,707 | 61,666 | 58,776 |
| cross: <niji_ld::elf::Elf>::check_table incl | 6,081 | 23,523 | 6,081 |
| cross: <niji_ld::elf::Elf>::parse::<niji_ld::arch::X86_64> incl | 1,833 | 21,507 | 1,833 |
