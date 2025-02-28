
if grep --color=always -RInE 'UNDONE\s*\(\s*\)\s*:?' crates; then
  echo "Error: Found unfinished work."
  exit 1
fi
