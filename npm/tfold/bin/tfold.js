#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");
const { createRequire } = require("node:module");
const { dirname, join } = require("node:path");

const NATIVE_PACKAGES = {
  "darwin-arm64": {
    packageName: "@konvert7/tfold-darwin-arm64",
    binaryPath: "bin/tfold",
  },
  "darwin-x64": {
    packageName: "@konvert7/tfold-darwin-x64",
    binaryPath: "bin/tfold",
  },
  "linux-x64": {
    packageName: "@konvert7/tfold-linux-x64",
    binaryPath: "bin/tfold",
  },
  "win32-x64": {
    packageName: "@konvert7/tfold-win32-x64",
    binaryPath: "bin/tfold.exe",
  },
};

function nativePackageForHost() {
  return NATIVE_PACKAGES[`${process.platform}-${process.arch}`];
}

function resolveBinary(nativePackage) {
  const requireFromShim = createRequire(join(__dirname, "..", "package.json"));
  try {
    const manifest = requireFromShim.resolve(`${nativePackage.packageName}/package.json`);
    return join(dirname(manifest), nativePackage.binaryPath);
  } catch {
    return undefined;
  }
}

function fail(message) {
  process.stderr.write(`tfold: ${message}\n`);
  process.exit(1);
}

function run() {
  const nativePackage = nativePackageForHost();
  if (!nativePackage) {
    fail(
      `no prebuilt binary for ${process.platform}-${process.arch}. Build it with: cargo install tfold`
    );
  }

  const binary = process.env.TFOLD_BINARY || resolveBinary(nativePackage);
  if (!binary) {
    fail(
      `${nativePackage.packageName} is not installed. It ships as an optional dependency, so reinstall without --no-optional, or build from source with: cargo install tfold`
    );
  }

  const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
  if (result.error) {
    fail(
      `could not run ${binary}: ${result.error.message}. On a musl-based system (Alpine), build from source with: cargo install tfold`
    );
  }
  process.exit(result.status === null ? 1 : result.status);
}

run();
