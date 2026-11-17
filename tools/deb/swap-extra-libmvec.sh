# tools/deb/swap-extra-libmvec.sh: extra commands for tools/deb/swap-system.sh (SWAP_EXTRA_CMDS), sourced by
# swap-cmds.sh (its `run` and `sh_` helpers). Two of the package-parity audit's sampled users of libmvec.so.1
# (results/package-parity/consumers-users.tsv): direwolf and cif2hkl. Install them with
# SWAP_EXTRA_PKGS=direwolf,cif2hkl. No spec clause: release validation (SYS-LIBMVEC, plan/package-parity-audit.md
# row 2). The outputs keep no timing or date lines, so the stock root and the libniji root compare line by line.
m=/tmp/mvec
rm -rf "$m"
mkdir -p "$m"
# Which of the programs load libmvec.so.1 (resolved to a file in both roots).
sh_ mvec-ldd 'for f in /usr/bin/gen_packets /usr/bin/atest /usr/bin/direwolf /usr/bin/cif2hkl; do printf "%s %s\n" "$f" "$(ldd "$f" 2>&1 | grep -c "libmvec\.so\.1 => /")"; done'
# direwolf: gen_packets writes AX.25 test frames as audio (sine tones), atest decodes the file.
sh_ mvec-direwolf-gen "gen_packets -o $m/p.wav -n 20 > $m/gen.log 2>&1; echo rc=\$?; wc -c < $m/p.wav"
sh_ mvec-direwolf-decode "atest $m/p.wav 2>&1 | grep -v -i -E 'seconds|elapsed|version|dire wolf'"
# cif2hkl: structure factors of rock salt from a small CIF (sin, cos and exp over every reflection).
printf '%s\n' 'data_NaCl' '_cell_length_a 5.64' '_cell_length_b 5.64' '_cell_length_c 5.64' \
    '_cell_angle_alpha 90' '_cell_angle_beta 90' '_cell_angle_gamma 90' "_symmetry_space_group_name_H-M 'F m -3 m'" \
    'loop_' '_atom_site_label' '_atom_site_type_symbol' '_atom_site_fract_x' '_atom_site_fract_y' '_atom_site_fract_z' \
    'Na1 Na 0 0 0' 'Cl1 Cl 0.5 0.5 0.5' > "$m/nacl.cif"
sh_ mvec-cif2hkl-powder "cif2hkl --mode NUC --powder --out $m/nacl-p.hkl $m/nacl.cif > $m/cif.log 2>&1; echo rc=\$?; grep -v '^#' $m/nacl-p.hkl | head -60"
sh_ mvec-cif2hkl-xtal "cif2hkl --mode XRA --xtal --lambda 1.54 --out $m/nacl-x.hkl $m/nacl.cif > $m/cif.log 2>&1; echo rc=\$?; grep -v '^#' $m/nacl-x.hkl | head -60"
