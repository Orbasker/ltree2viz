#!/usr/bin/env node
// Assembles the npm packages from the binaries the `dist` release built, and
// (when NPM_PUBLISH=1) publishes them: every platform package first, then the
// root package last — the root's optionalDependencies must already resolve on
// the registry or a fresh install breaks.
//
// Inputs (env):
//   VERSION        crate/tag version, no leading `v` (required)
//   TAG            git tag to download release archives from (default: v$VERSION)
//   NPM_PUBLISH    "1" to actually publish; otherwise assemble-only (dry run)
//
// Downloads archives with `gh release download`, so GH_TOKEN must be set in CI.

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync, copyFileSync, chmodSync, readdirSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const BUILD_DIR = join(HERE, "build");
const DOWNLOAD_DIR = join(BUILD_DIR, "archives");

const APP = "ltree2viz";
const REPO = "https://github.com/Orbasker/ltree2viz";
const DESCRIPTION = "Visualize a Postgres ltree hierarchy as a Mermaid diagram or interactive HTML tree";
const LICENSE = "MIT OR Apache-2.0";

// One entry per platform package. `target` is the Rust target triple dist built.
const PLATFORMS = [
  { target: "aarch64-apple-darwin", os: "darwin", cpu: "arm64", exe: "ltree2viz" },
  { target: "x86_64-apple-darwin", os: "darwin", cpu: "x64", exe: "ltree2viz" },
  { target: "x86_64-unknown-linux-musl", os: "linux", cpu: "x64", exe: "ltree2viz" },
  { target: "aarch64-unknown-linux-musl", os: "linux", cpu: "arm64", exe: "ltree2viz" },
  { target: "x86_64-pc-windows-msvc", os: "win32", cpu: "x64", exe: "ltree2viz.exe" },
];

const version = required("VERSION").replace(/^v/, "");
const tag = process.env.TAG || `v${version}`;
const publish = process.env.NPM_PUBLISH === "1";

function required(name) {
  const value = process.env[name];
  if (!value) {
    console.error(`build.mjs: missing required env var ${name}`);
    process.exit(1);
  }
  return value;
}

function run(cmd, args, opts = {}) {
  return execFileSync(cmd, args, { stdio: "inherit", ...opts });
}

// dist archives are `ltree2viz-<target>.tar.xz` (unix) / `.zip` (windows) and
// unpack to a directory containing the binary somewhere inside.
function archiveName(platform) {
  const ext = platform.os === "win32" ? "zip" : "tar.xz";
  return `${APP}-${platform.target}.${ext}`;
}

function findBinary(dir, exe) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      const found = findBinary(full, exe);
      if (found) return found;
    } else if (entry === exe) {
      return full;
    }
  }
  return null;
}

function extractBinary(platform) {
  const archive = join(DOWNLOAD_DIR, archiveName(platform));
  const dest = join(DOWNLOAD_DIR, platform.target);
  rmSync(dest, { recursive: true, force: true });
  mkdirSync(dest, { recursive: true });
  if (platform.os === "win32") {
    run("unzip", ["-q", "-o", archive, "-d", dest]);
  } else {
    run("tar", ["-xf", archive, "-C", dest]);
  }
  const binary = findBinary(dest, platform.exe);
  if (!binary) {
    console.error(`build.mjs: ${platform.exe} not found inside ${archive}`);
    process.exit(1);
  }
  return binary;
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function assemblePlatform(platform) {
  const name = `@ltree2viz/${platform.os}-${platform.cpu}`;
  const pkgDir = join(BUILD_DIR, `${platform.os}-${platform.cpu}`);
  rmSync(pkgDir, { recursive: true, force: true });
  mkdirSync(pkgDir, { recursive: true });

  const binary = extractBinary(platform);
  const destBinary = join(pkgDir, platform.exe);
  copyFileSync(binary, destBinary);
  chmodSync(destBinary, 0o755);

  writeJson(join(pkgDir, "package.json"), {
    name,
    version,
    description: `${DESCRIPTION} (${platform.os}-${platform.cpu} binary)`,
    license: LICENSE,
    repository: { type: "git", url: `git+${REPO}.git` },
    os: [platform.os],
    cpu: [platform.cpu],
    files: [platform.exe],
  });

  return { name, dir: pkgDir };
}

function assembleRoot() {
  const pkgDir = join(BUILD_DIR, "ltree2viz");
  rmSync(pkgDir, { recursive: true, force: true });
  mkdirSync(join(pkgDir, "bin"), { recursive: true });

  copyFileSync(join(HERE, "bin", "ltree2viz"), join(pkgDir, "bin", "ltree2viz"));
  chmodSync(join(pkgDir, "bin", "ltree2viz"), 0o755);
  copyFileSync(join(HERE, "README.md"), join(pkgDir, "README.md"));

  const optionalDependencies = {};
  for (const platform of PLATFORMS) {
    optionalDependencies[`@ltree2viz/${platform.os}-${platform.cpu}`] = version;
  }

  writeJson(join(pkgDir, "package.json"), {
    name: APP,
    version,
    description: DESCRIPTION,
    license: LICENSE,
    repository: { type: "git", url: `git+${REPO}.git` },
    homepage: `${REPO}#readme`,
    keywords: [
      "postgres",
      "postgresql",
      "ltree",
      "hierarchy",
      "tree",
      "mermaid",
      "diagram",
      "visualization",
      "html",
      "cli",
    ],
    bin: { ltree2viz: "bin/ltree2viz" },
    files: ["bin"],
    engines: { node: ">=14" },
    optionalDependencies,
  });

  return { name: APP, dir: pkgDir };
}

function npmPublish(pkg) {
  if (!publish) {
    console.log(`  (dry run) would publish ${pkg.name} from ${pkg.dir}`);
    return;
  }
  console.log(`  publishing ${pkg.name}`);
  run("npm", ["publish", "--access", "public"], { cwd: pkg.dir });
}

// --- main ---------------------------------------------------------------

rmSync(BUILD_DIR, { recursive: true, force: true });
mkdirSync(DOWNLOAD_DIR, { recursive: true });

console.log(`Downloading release archives for ${tag}`);
const patterns = PLATFORMS.flatMap((p) => ["--pattern", archiveName(p)]);
run("gh", ["release", "download", tag, "--dir", DOWNLOAD_DIR, "--clobber", ...patterns]);

console.log("Assembling platform packages");
const platformPkgs = PLATFORMS.map(assemblePlatform);

console.log("Assembling root package");
const rootPkg = assembleRoot();

console.log(publish ? "Publishing packages" : "Dry run (set NPM_PUBLISH=1 to publish)");
for (const pkg of platformPkgs) npmPublish(pkg);
npmPublish(rootPkg); // root last: its optionalDependencies must already resolve

console.log("Done");
