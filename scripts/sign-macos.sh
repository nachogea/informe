#!/bin/sh
# Run after bundling and before archiving. Credentials remain in Keychain.
set -eu
: "${SIGNING_IDENTITY:?Set the Developer ID Application identity}"
: "${NOTARY_PROFILE:?Set an existing notarytool Keychain profile}"
bundle="${1:-target/release/Informe.app}"
codesign --force --options runtime --timestamp --sign "$SIGNING_IDENTITY" "$bundle"
codesign --verify --strict --verbose=2 "$bundle"
submission=$(mktemp -d)
trap 'rm -rf "$submission"' EXIT
ditto -c -k --keepParent "$bundle" "$submission/informe.zip"
xcrun notarytool submit "$submission/informe.zip" --keychain-profile "$NOTARY_PROFILE" --wait
xcrun stapler staple "$bundle"
xcrun stapler validate "$bundle"
spctl --assess --type execute --verbose "$bundle"
