#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Load .env if it exists
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Check required env vars
if [ -z "$CLOUDFLARE_API_TOKEN" ]; then
    echo "Missing CLOUDFLARE_API_TOKEN"
    echo "Set it in landing/.env or export it"
    exit 1
fi

if [ -z "$CLOUDFLARE_ACCOUNT_ID" ]; then
    echo "Missing CLOUDFLARE_ACCOUNT_ID"
    echo "Set it in landing/.env or export it"
    exit 1
fi

DEPLOY_TARGET="${1:-all}"

deploy_r8r() {
    echo "Deploying r8r..."
    wrangler pages deploy "$SCRIPT_DIR/r8r" \
        --project-name=r8r --branch=main --commit-dirty=true
    echo "Done: https://r8r.pages.dev"
}

deploy_hmanlab() {
    echo "Building hmanlab docs..."
    cd "$SCRIPT_DIR/hmanlab/docs"
    rm -rf dist .astro
    npm install --silent
    npx astro build
    cd "$SCRIPT_DIR"

    echo "Assembling deploy..."
    rm -rf "$SCRIPT_DIR/hmanlab/_deploy"
    mkdir -p "$SCRIPT_DIR/hmanlab/_deploy/docs"
    cp "$SCRIPT_DIR/hmanlab/index.html" "$SCRIPT_DIR/hmanlab/_deploy/"
    cp "$SCRIPT_DIR/hmanlab/mascot-no-bg.png" "$SCRIPT_DIR/hmanlab/_deploy/"
    cp "$SCRIPT_DIR/../deploy/setup.sh" "$SCRIPT_DIR/hmanlab/_deploy/"
    [ -f "$SCRIPT_DIR/hmanlab/favicon.svg" ] && cp "$SCRIPT_DIR/hmanlab/favicon.svg" "$SCRIPT_DIR/hmanlab/_deploy/"
    cp -r "$SCRIPT_DIR/hmanlab/docs/dist/"* "$SCRIPT_DIR/hmanlab/_deploy/docs/"

    echo "Deploying hmanlab..."
    wrangler pages deploy "$SCRIPT_DIR/hmanlab/_deploy" \
        --project-name=hmanlab --branch=main --commit-dirty=true
    rm -rf "$SCRIPT_DIR/hmanlab/_deploy"
    echo "Done: https://hmanlab.com"
}

case "$DEPLOY_TARGET" in
    hmanlab) deploy_hmanlab ;;
    r8r)       deploy_r8r ;;
    all)       deploy_r8r; echo ""; deploy_hmanlab ;;
    *)         echo "Usage: deploy.sh [hmanlab|r8r|all]"; exit 1 ;;
esac
