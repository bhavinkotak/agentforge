#!/usr/bin/env bash
# push-to-github.sh
# Run this once to create a private GitHub repo and push.
# Usage: GITHUB_TOKEN=ghp_xxx GITHUB_USER=yourname bash push-to-github.sh

set -euo pipefail

: "${GITHUB_TOKEN:?Set GITHUB_TOKEN to a token with 'repo' scope}"
: "${GITHUB_USER:?Set GITHUB_USER to your GitHub username}"

REPO_NAME="agentforge"

echo "Creating private repo ${GITHUB_USER}/${REPO_NAME}..."
curl -sf -X POST \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github+json" \
  https://api.github.com/user/repos \
  -d "{\"name\":\"${REPO_NAME}\",\"private\":true,\"description\":\"Self-improving AI agent optimization platform. One file in, a better agent out.\"}" \
  > /dev/null

echo "Repo created. Pushing..."
git remote add origin "https://${GITHUB_USER}:${GITHUB_TOKEN}@github.com/${GITHUB_USER}/${REPO_NAME}.git"
git push -u origin main

echo ""
echo "Done! Visit: https://github.com/${GITHUB_USER}/${REPO_NAME}"
