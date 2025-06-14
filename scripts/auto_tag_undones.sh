#!/bin/bash
# Requirements: gh (GitHub CLI), sed, grep

git_commit=$(git rev-parse HEAD)
# Automatically tags untagged undones
grep -RInE 'UNDONE\s*\(\s*\)\s*:?' crates --exclude=.git/ --exclude=target/ | while IFS=: read -r file lineno linecontent; do
    task=$(echo "$linecontent" | sed -nE 's/.*UNDONE\s*\(\s*\)\s*:?[[:space:]]*(.*)/\1/p')
    [ -z "$task" ] && task="No description provided"

    echo "Creating issue for $file:$lineno – $task:"
    start=$(( lineno - 3 ))
    [ "$start" -lt 1 ] && start=1
    end=$(( lineno + 3 ))

    body=$(printf "https://github.com/obliviouslabs/rostl/blob/%s/%s#L%s-L%s\n Task: %s" "$git_commit" "$file" "$start" "$end" "$task")
    # Create the GitHub issue (which outputs a URL ending with the issue number).
    issue_output=$(gh issue create \
                   --title "UNDONE(): $task" \
                   --body "$body")

    # Extract the issue number from the issue URL.
    issue_number=$(echo "$issue_output" | grep -oE '[0-9]+$')

    if [ -z "$issue_number" ]; then
        echo "Failed to extract issue number for $file:$lineno. Output was: $issue_output"
        continue
    fi

    echo "$issue_output"

    # echo "Tagging $file:$lineno with ID git-$issue_number: "
    # Update the file by bt tagging the respective undone
    sed -i -E "${lineno}s/UNDONE\\s*\\(\\s*\\)\\s*:?/UNDONE(git-$issue_number):/" "$file"
done
