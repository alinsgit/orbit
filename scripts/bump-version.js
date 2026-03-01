#!/usr/bin/env node
/**
 * Orbit version bump script (cross-platform)
 * Usage: node scripts/bump-version.js 1.4.0
 *    or: bun run bump 1.4.0
 */

const fs = require('fs');
const path = require('path');

const version = process.argv[2];

if (!version) {
    console.error('Usage: node scripts/bump-version.js <version>');
    console.error('Example: node scripts/bump-version.js 1.4.0');
    process.exit(1);
}

if (!/^\d+\.\d+\.\d+$/.test(version)) {
    console.error('Error: Invalid version format. Expected X.Y.Z (e.g., 1.4.0)');
    process.exit(1);
}

const root = path.join(__dirname, '..');

function bumpJson(filePath, ...keys) {
    const raw = fs.readFileSync(filePath, 'utf8');
    const data = JSON.parse(raw);
    let target = data;
    for (let i = 0; i < keys.length - 1; i++) target = target[keys[i]];
    target[keys[keys.length - 1]] = version;
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n');
    console.log(`  Updated ${path.relative(root, filePath)}`);
}

function bumpToml(filePath) {
    let content = fs.readFileSync(filePath, 'utf8');
    // Replace only the first occurrence (package version, not dependencies)
    content = content.replace(/^version = "[\d.]+"/m, `version = "${version}"`);
    fs.writeFileSync(filePath, content);
    console.log(`  Updated ${path.relative(root, filePath)}`);
}

console.log(`Bumping version to ${version}...\n`);

bumpToml(path.join(root, 'core', 'Cargo.toml'));
bumpJson(path.join(root, 'package.json'), 'version');
bumpJson(path.join(root, 'core', 'tauri.conf.json'), 'version');

console.log(`
Version bumped to ${version} in all 3 files.
Next steps:
  git add core/Cargo.toml package.json core/tauri.conf.json
  git commit -m "bump: v${version}"
  git tag v${version}
  git push && git push origin v${version}
`);
