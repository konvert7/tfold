#!/usr/bin/env node
import { chmodSync, copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import {
  assertManifestsMatchShimTable,
  fail,
  NATIVE_PACKAGES,
  nativeManifestPath,
  npmRoot,
  repoRoot,
  shimManifestPath,
  stampedManifest,
  writeJson,
} from "./native-packages.mjs";

function valueAfter(args, name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

function parseArgs() {
  const args = process.argv.slice(2);
  const version = valueAfter(args, "--version");
  if (!version) {
    fail(
      "Usage: node npm/prepare-release.mjs --version <version> --binaries <dir> [--out <dir>] [--allow-missing]"
    );
  }
  if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
    fail(`Version ${version} is not semver`);
  }
  return {
    version,
    binariesDir: resolve(
      valueAfter(args, "--binaries") ?? join(repoRoot, "dist", "binaries")
    ),
    outDir: resolve(valueAfter(args, "--out") ?? join(repoRoot, "dist", "release")),
    allowMissing: args.includes("--allow-missing"),
  };
}

function stagedBinaryPath(options, key) {
  return join(options.binariesDir, key, NATIVE_PACKAGES[key].binaryPath.split("/").pop());
}

function prepareNativePackage(options, key) {
  const nativePackage = NATIVE_PACKAGES[key];
  const outDir = join(options.outDir, "native", key);
  writeJson(
    join(outDir, "package.json"),
    stampedManifest(nativeManifestPath(key), options.version, { publishable: true })
  );

  const source = stagedBinaryPath(options, key);
  if (!existsSync(source))
    return { key, packageName: nativePackage.packageName, binaryCopied: false };

  const target = join(outDir, nativePackage.binaryPath);
  mkdirSync(dirname(target), { recursive: true });
  copyFileSync(source, target);
  chmodSync(target, 0o755);
  return { key, packageName: nativePackage.packageName, binaryCopied: true };
}

function prepareShimPackage(options) {
  const outDir = join(options.outDir, "tfold");
  const manifest = stampedManifest(shimManifestPath(), options.version);
  if (
    Object.keys(manifest.optionalDependencies ?? {}).length !==
    Object.keys(NATIVE_PACKAGES).length
  ) {
    fail("Shim manifest lost an optional dependency during stamping");
  }
  writeJson(join(outDir, "package.json"), manifest);
  mkdirSync(join(outDir, "bin"), { recursive: true });
  copyFileSync(
    join(npmRoot, "tfold", "bin", "tfold.js"),
    join(outDir, "bin", "tfold.js")
  );
  copyFileSync(join(repoRoot, "README.md"), join(outDir, "README.md"));
  return manifest;
}

const options = parseArgs();
assertManifestsMatchShimTable();

rmSync(options.outDir, { recursive: true, force: true });
mkdirSync(options.outDir, { recursive: true });

const natives = Object.keys(NATIVE_PACKAGES).map((key) =>
  prepareNativePackage(options, key)
);
const shim = prepareShimPackage(options);
const missing = natives.filter((native) => !native.binaryCopied);

if (missing.length > 0 && !options.allowMissing) {
  fail(`Missing native binaries for: ${missing.map((native) => native.key).join(", ")}`);
}

process.stdout.write(
  [
    `prepared ${shim.name}@${shim.version} in ${options.outDir}`,
    `native packages: ${natives.length - missing.length} of ${natives.length} with binaries`,
    missing.length > 0
      ? `missing: ${missing.map((native) => native.key).join(", ")}`
      : "missing: none",
    `publish order: ${natives.map((native) => native.packageName).join(", ")}, then ${shim.name}`,
    "",
  ].join("\n")
);
