import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const CRATE_NAME = "tfold";

export async function verifyConditions(_, context) {
  const manifest = readManifest(cwdOf(context));
  if (crateNameOf(manifest) !== CRATE_NAME) {
    throw new Error(`Cargo.toml declares ${crateNameOf(manifest)}, expected ${CRATE_NAME}`);
  }
}

export async function prepare(pluginConfig = {}, context) {
  const cwd = cwdOf(context);
  const version = versionOf(context);

  writeManifest(cwd, stampVersion(readManifest(cwd), version));
  refreshLockfile(cwd, pluginConfig.spawnSync ?? spawnSync);

  context.logger?.log(`Stamped Cargo.toml and Cargo.lock at ${version}`);
}

function stampVersion(manifest, version) {
  return manifest.replace(/^version = ".*"$/m, `version = "${version}"`);
}

function refreshLockfile(cwd, spawn) {
  const result = spawn("cargo", ["update", "--workspace"], {
    cwd,
    encoding: "utf-8",
    stdio: ["ignore", "ignore", "pipe"],
  });
  if ((result.status ?? -1) !== 0) {
    throw new Error(`cargo update failed with exit code ${result.status ?? -1}\n${result.stderr ?? ""}`);
  }
}

function crateNameOf(manifest) {
  return manifest.match(/^name = "(.*)"$/m)?.[1];
}

function manifestPath(cwd) {
  return join(cwd, "Cargo.toml");
}

function readManifest(cwd) {
  return readFileSync(manifestPath(cwd), "utf-8");
}

function writeManifest(cwd, manifest) {
  writeFileSync(manifestPath(cwd), manifest);
}

function cwdOf(context) {
  return context.cwd ?? process.cwd();
}

function versionOf(context) {
  const version = context.nextRelease?.version;
  if (!version) throw new Error("semantic-release did not provide nextRelease.version");
  return version;
}
