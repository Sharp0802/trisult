#!/bin/sh

echo "# Changelog"
echo

git tag | sort -r | {
    read -r current_tag

    while [ -n "$current_tag" ]; do
        if read -r next_tag; then
            start="${next_tag}.."
        else
            start=""
            next_tag=""
        fi

        echo "## ${current_tag}"
        echo

        git log "${start}${current_tag}" --oneline --no-decorate \
            | sort -k2 \
            | awk '{printf "- `%s` %s\n", $1, substr($0, index($0, $2))}'

        echo

        current_tag="$next_tag"
    done
}
