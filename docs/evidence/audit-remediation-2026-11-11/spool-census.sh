#!/usr/bin/env bash
# Exact, reproducible epoch-spool census. Prints: repo <TAB> spools <TAB> MiB <TAB> gitdir
set -u
for r in "$@"; do
  gd=$(git -C "$r" rev-parse --path-format=absolute --git-dir 2>/dev/null) || gd="$r/.git"
  n=$(ls -d "$gd"/dracon-staging-* 2>/dev/null | wc -l)
  k=$(du -sk "$gd"/dracon-staging-* 2>/dev/null | awk '{s+=$1} END {print s+0}')
  mi=$(awk -v k="$k" 'BEGIN{printf "%.1f", k/1024}')
  [ "$n" -gt 0 ] && printf '%s\t%s\t%s\t%s\n' "$r" "$n" "$mi" "$gd"
done
