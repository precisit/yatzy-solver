#!/usr/bin/env bash
# Is a version already published? Used by the release workflow so that a publish job can be re-run after a
# partial failure: each registry step skips what is already there.
#
#   release/published.sh crates|pypi|npm|github NAME VERSION     (for github, NAME is the tag)
#
# Exit 0: published. Exit 1: not published. Exit 2: could not tell (network or registry error).
# PRETEND_PUBLISHED="crates:yatzy-solver@1.0.0 npm:yatzy-solver@1.0.0" makes the listed entries count as
# published, for testing the skip logic in dry runs.
set -u
if [ $# -ne 3 ]; then echo "usage: $0 crates|pypi|npm|github NAME VERSION" >&2; exit 2; fi
kind=$1 name=$2 version=$3
for p in ${PRETEND_PUBLISHED:-}; do
  if [ "$p" = "$kind:$name@$version" ]; then echo "pretend: $kind $name $version is published"; exit 0; fi
done
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
case $kind in
  crates)
    # The sparse index: a name of four or more characters lives at <first two>/<next two>/<name>.
    lower=$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]')
    code=$(curl -sS -o "$tmp" -w '%{http_code}' "https://index.crates.io/${lower:0:2}/${lower:2:2}/$lower") || exit 2
    [ "$code" = 404 ] && exit 1
    [ "$code" = 200 ] || exit 2
    grep -q "\"vers\":\"$version\"" "$tmp" && exit 0 || exit 1
    ;;
  pypi)
    code=$(curl -sS -o /dev/null -w '%{http_code}' "https://pypi.org/pypi/$name/$version/json") || exit 2
    [ "$code" = 200 ] && exit 0
    [ "$code" = 404 ] && exit 1
    exit 2
    ;;
  npm)
    out=$(npm view "$name@$version" version 2>"$tmp")
    [ "$out" = "$version" ] && exit 0
    grep -q "E404" "$tmp" && exit 1
    [ -z "$out" ] && ! grep -q "ERR" "$tmp" && exit 1
    exit 2
    ;;
  github)
    gh release view "$name" > /dev/null 2> "$tmp" && exit 0
    grep -qi "not found" "$tmp" && exit 1
    exit 2
    ;;
  *)
    echo "unknown registry $kind" >&2
    exit 2
    ;;
esac
