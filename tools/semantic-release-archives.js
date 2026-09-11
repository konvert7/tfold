import { spawnSync } from "node:child_process";
import { mkdirSync, copyFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { NATIVE_PACKAGES } from "../npm/native-packages.mjs";

export async function prepare(pluginConfig = {}, context) {
  const cwd = cwdOf(context);
  const version = versionOf(context);
  const spawn = pluginConfig.spawnSync ?? spawnSync;
  const archives = archivesDir(cwd);

  rmSync(archives, { recursive: true, force: true });
  mkdirSync(archives, { recursive: true });

  for (const key of Object.keys(NATIVE_PACKAGES)) {
    packOnePlatform(spawn, cwd, archives, version, key);
  }
  writeChecksums(spawn, archives, version);

  context.logger?.log(`Packed ${readdirSync(archives).length} release artifacts for ${version}`);
}

function packOnePlatform(spawn, cwd, archives, version, key) {
  const name = `tfold-${version}-${key}`;
  const staging = join(archives, name);
  mkdirSync(staging, { recursive: true });

  const binary = NATIVE_PACKAGES[key].binaryPath.split("/").pop();
  copyFileSync(join(cwd, "dist", "binaries", key, binary), join(staging, binary));
  copyFileSync(join(cwd, "README.md"), join(staging, "README.md"));

  run(spawn, "tar", ["-czf", join(archives, `${name}.tar.gz`), "-C", archives, name]);
  rmSync(staging, { recursive: true, force: true });
}

function writeChecksums(spawn, archives, version) {
  const tarballs = readdirSync(archives).filter((entry) => entry.endsWith(".tar.gz"));
  const result = run(spawn, "shasum", ["-a", "256", ...tarballs], archives);
  writeFileSync(join(archives, `tfold-${version}-SHA256SUMS`), result.stdout);
}

function run(spawn, command, args, cwd) {
  const result = spawn(command, args, { cwd, encoding: "utf-8" });
  if ((result.status ?? -1) !== 0) {
    throw new Error(`${command} failed with exit code ${result.status ?? -1}\n${result.stderr ?? ""}`);
  }
  return result;
}

function archivesDir(cwd) {
  return join(cwd, "dist", "archives");
}

function cwdOf(context) {
  return context.cwd ?? process.cwd();
}

function versionOf(context) {
  const version = context.nextRelease?.version;
  if (!version) throw new Error("semantic-release did not provide nextRelease.version");
  return version;
}
