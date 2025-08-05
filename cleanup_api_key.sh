#!/bin/bash

# Script to remove API key from git history
# This will rewrite history to remove the sensitive API key

API_KEY="e255da881e8849e39bceca5362b30cb3.SBnj4V3WO3BvEbvv"

echo "Starting API key removal from git history..."
echo "API key to remove: $API_KEY"

# Create a backup branch
git checkout -b backup-before-cleanup
echo "✅ Created backup branch: backup-before-cleanup"

# Go back to master
git checkout master

# Use git filter-branch to remove the API key from all commits
git filter-branch --force --index-filter \
    'git rm --cached --ignore-unmatch config.json 2>/dev/null || git ls-files -z | xargs -0 grep -lZ "e255da881e8849e39bceca5362b30cb3.SBnj4V3WO3BvEbvv" | xargs -0 rm -f 2>/dev/null || true' \
    --prune-empty --tag-name-filter cat -- --all

echo "✅ Filter-branch completed"

# Force push the changes (this will rewrite history)
echo "⚠️  WARNING: This will rewrite git history and force push to remote"
echo "Make sure you have committed the current config.json with empty API key"
read -p "Press Enter to continue with force push, or Ctrl+C to cancel..."

# If we get here, user wants to proceed
# Note: In a real scenario, you would force push to remote
# git push --force --all
# git push --force --tags

echo "✅ API key removal completed"
echo "Don't forget to force push to remote repository:"
echo "  git push --force --all"
echo "  git push --force --tags"