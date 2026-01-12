# Repo-local bd alias to avoid macOS cache permission issues in sandboxed shells
# Usage: source this file in your shell (or add the line to your rc file).
alias bdr='(cd /Users/briansquires/code/rust-daq && HOME=/Users/briansquires/code/rust-daq bd --sandbox "$@")'
