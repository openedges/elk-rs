# Re-joins Rust source lines that rustfmt wrapped, so that the line-oriented awk
# parsers in scripts/check_algorithm_*_parity.sh see each binding and each
# method-chain receiver on a single line:
#
#   let mut data =                                  let mut data = LayoutAlgorithmData::new(...)
#       LayoutAlgorithmData::new(...);        =>
#
#   random_data                                     random_data.add_known_option_default(...)
#       .add_known_option_default(...);       =>
#
# Only the two shapes above are joined; every other line is passed through as is.
# Usage: awk -f scripts/lib/join_wrapped_rust_lines.awk file.rs | awk '...'

function ltrim(text) {
    sub(/^[[:space:]]+/, "", text)
    return text
}

{
    line = $0
    if (held != "") {
        if (held_kind == "assign") {
            line = held " " ltrim(line)
        } else if (line ~ /^[[:space:]]*\./) {
            line = held ltrim(line)
        } else {
            print held
        }
        held = ""
        held_kind = ""
    }
    if (line ~ /^[[:space:]]*let[[:space:]]+(mut[[:space:]]+)?[A-Za-z_][A-Za-z0-9_]*[[:space:]]*(:[^=]*)?=[[:space:]]*$/) {
        held = line
        held_kind = "assign"
        next
    }
    if (line ~ /^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]*$/) {
        held = line
        held_kind = "receiver"
        next
    }
    print line
}

END {
    if (held != "") {
        print held
    }
}
