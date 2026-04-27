#!/usr/bin/env node
/**
 * Orbit version bump + release driver (cross-platform).
 *
 * Usage:
 *   node scripts/bump-version.cjs <version>                  # update files only
 *   node scripts/bump-version.cjs <version> --commit         # + git commit + tag
 *   node scripts/bump-version.cjs <version> --commit --push  # + git push origin v<version>
 *   node scripts/bump-version.cjs <version> --release        # alias for --commit --push
 *   node scripts/bump-version.cjs <version> --tagline "..."  # rewrite hero-badge tagline
 *
 * Exposed via package.json:
 *   bun run bump 1.2.0             # files only
 *   bun run release 1.2.0          # full flow (bump → commit → tag → push)
 *
 * On `--push`, CI on tag push (`v*`) builds installers and publishes a
 * GitHub Release; that's the trigger Orbit's release flow relies on.
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// ── Args ────────────────────────────────────────────────────────────
// Position-agnostic so it works whether the package.json wrapper inserts
// flags before or after the user's positional version arg, e.g.
//   bun run release 1.5.0   ->  node scripts/bump-version.cjs --release 1.5.0
const argv = process.argv.slice(2);
if (argv.length === 0 || argv.includes('--help') || argv.includes('-h')) {
  printUsage();
  process.exit(argv.length === 0 ? 1 : 0);
}

// First non-flag, non-tagline-value arg is the version.
let version = null;
const flags = new Set();
let tagline = null;
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a === '--tagline') {
    if (argv[i + 1] && !argv[i + 1].startsWith('--')) {
      tagline = argv[++i];
    }
    continue;
  }
  if (a.startsWith('--')) {
    flags.add(a);
    continue;
  }
  if (version === null) {
    version = a;
  }
}
if (flags.has('--release')) {
  flags.add('--commit');
  flags.add('--push');
}

if (!version) {
  console.error('Error: missing version argument.');
  printUsage();
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error('Error: Invalid version format. Expected X.Y.Z (e.g., 1.4.0)');
  process.exit(1);
}

const root = path.join(__dirname, '..');

// ── File updaters ───────────────────────────────────────────────────
function bumpJson(filePath, ...keys) {
  const raw = fs.readFileSync(filePath, 'utf8');
  const data = JSON.parse(raw);
  let target = data;
  for (let i = 0; i < keys.length - 1; i++) target = target[keys[i]];
  target[keys[keys.length - 1]] = version;
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n');
  console.log(`  ✓ ${path.relative(root, filePath)}`);
}

function bumpToml(filePath) {
  let content = fs.readFileSync(filePath, 'utf8');
  // Replace only the first `version = "X.Y.Z"` (package version, never deps).
  // The `m` flag anchors `^` to line start so dependency lines like
  // `serde = { version = "1" }` are skipped.
  const before = content;
  content = content.replace(/^version = "[\d.]+"/m, `version = "${version}"`);
  if (content === before) {
    throw new Error(`Did not find package version line in ${filePath}`);
  }
  fs.writeFileSync(filePath, content);
  console.log(`  ✓ ${path.relative(root, filePath)}`);
}

/// Updates the hero-badge "vX.Y.Z — <tagline>" string in docs/index.html.
/// If `--tagline "..."` was supplied the tagline is rewritten too; otherwise
/// only the version segment changes and the user's existing tagline is kept.
function bumpDocsBadge(filePath) {
  if (!fs.existsSync(filePath)) {
    console.log(`  · ${path.relative(root, filePath)} (skipped — not found)`);
    return;
  }
  let content = fs.readFileSync(filePath, 'utf8');
  // Match: <span>vMAJ.MIN.PATCH — anything-up-to-</span>
  // We capture the prose after the em-dash so we can preserve it.
  const re = /<span>v\d+\.\d+\.\d+\s*—\s*([^<]*)<\/span>/;
  const match = content.match(re);
  if (!match) {
    console.log(
      `  · ${path.relative(root, filePath)} (no hero badge found — left untouched)`
    );
    return;
  }
  const newTagline = tagline ?? match[1].trim();
  content = content.replace(
    re,
    `<span>v${version} — ${newTagline}</span>`
  );
  fs.writeFileSync(filePath, content);
  console.log(
    `  ✓ ${path.relative(root, filePath)}` +
      (tagline ? ` (tagline rewritten)` : ` (tagline preserved)`)
  );
}

// ── Run file edits ──────────────────────────────────────────────────
console.log(`Bumping to v${version}…\n`);
bumpToml(path.join(root, 'core', 'Cargo.toml'));
bumpJson(path.join(root, 'package.json'), 'version');
bumpJson(path.join(root, 'core', 'tauri.conf.json'), 'version');
bumpDocsBadge(path.join(root, 'docs', 'index.html'));

// ── Optional: commit + tag + push ───────────────────────────────────
if (!flags.has('--commit') && !flags.has('--push')) {
  console.log(`
Files updated. Next:
  git add -A
  git commit -m "bump: v${version}"
  git tag -a v${version} -m "Orbit v${version}"
  git push && git push origin v${version}

Or run:
  bun run release ${version}      # commit + tag + push (CI triggers)
`);
  process.exit(0);
}

function run(cmd, opts = {}) {
  console.log(`  $ ${cmd}`);
  execSync(cmd, { stdio: 'inherit', cwd: root, ...opts });
}

if (flags.has('--commit')) {
  console.log('\nCommitting…');
  // Stage exactly the files we modified — never `git add -A`, which would
  // sweep in unrelated work-in-progress.
  const staged = [
    'core/Cargo.toml',
    'package.json',
    'core/tauri.conf.json',
    'docs/index.html',
  ].filter((rel) => fs.existsSync(path.join(root, rel)));
  run(`git add ${staged.join(' ')}`);

  // Reject if nothing actually changed (e.g. running release on already-bumped tree).
  try {
    execSync('git diff --cached --quiet', { cwd: root });
    console.error(
      '\nError: nothing to commit — version files are already at the requested value.'
    );
    process.exit(1);
  } catch {
    // exit code 1 means "there is staged diff" — good
  }

  run(`git commit -m "bump: v${version}"`);
  run(`git tag -a v${version} -m "Orbit v${version}"`);
}

if (flags.has('--push')) {
  console.log('\nPushing…');
  run('git push');
  run(`git push origin v${version}`);
  console.log(`
✓ Tag v${version} pushed. CI will build & publish the release.
  Watch: https://github.com/alinsgit/orbit/actions
  Once CI finishes:
  https://github.com/alinsgit/orbit/releases/tag/v${version}
`);
} else if (flags.has('--commit')) {
  console.log(`
✓ Committed and tagged v${version} locally. Push when ready:
  git push && git push origin v${version}
`);
}

// ── Help ────────────────────────────────────────────────────────────
function printUsage() {
  console.log(`Orbit version bump + release driver

Usage:
  node scripts/bump-version.cjs <version> [flags]

Flags:
  --commit            git commit + git tag (no push)
  --push              git push + git push origin v<version> (requires --commit)
  --release           shortcut for --commit --push (CI triggers on tag push)
  --tagline "..."     rewrite the hero-badge tagline in docs/index.html
                      (preserves existing tagline if omitted)

Examples:
  bun run bump 1.5.0
  bun run release 1.5.0
  bun run release 1.5.0 --tagline "Multi-version services & junction layout"
`);
}
