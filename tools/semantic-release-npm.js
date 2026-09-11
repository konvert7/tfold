import { spawnSync } from "node:child_process";
import { join } from "node:path";
import {
  assertManifestsMatchShimTable,
  NATIVE_PACKAGES,
} from "../npm/native-packages.mjs";

export async function verifyConditions() {
  assertManifestsMatchShimTable();
}

export async function prepare(pluginConfig = {}, context) {
  const cwd = cwdOf(context);
  const spawn = pluginConfig.spawnSync ?? spawnSync;

  const result = spawn(
    "node",
    [
      "npm/prepare-release.mjs",
      "--version",
      versionOf(context),
      "--binaries",
      binariesDir(cwd),
      "--out",
      releaseDir(cwd),
    ],
    { cwd, encoding: "utf-8", stdio: ["ignore", "pipe", "pipe"] }
  );

  if ((result.status ?? -1) !== 0) {
    throw new Error(
      `prepare-release failed with exit code ${result.status ?? -1}\n${result.stderr ?? ""}`
    );
  }
  context.logger?.log(result.stdout?.trim());
}

export async function publish(pluginConfig = {}, context) {
  const cwd = cwdOf(context);
  const spawn = pluginConfig.spawnSync ?? spawnSync;

  if (context.options?.dryRun) {
    context.logger?.log("Dry run: skipping npm publishes");
    return;
  }

  for (const key of Object.keys(NATIVE_PACKAGES)) {
    publishPackage(
      spawn,
      join(releaseDir(cwd), "native", key),
      NATIVE_PACKAGES[key].packageName,
      context
    );
  }
  publishPackage(spawn, join(releaseDir(cwd), "tfold"), "tfold", context);
}

function publishPackage(spawn, dir, packageName, context) {
  const result = spawn("npm", ["publish", "--access", "public"], {
    cwd: dir,
    encoding: "utf-8",
    stdio: ["ignore", "pipe", "pipe"],
  });

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if ((result.status ?? -1) === 0) return;

  if (isUnconfiguredTrustedPublisher(result)) {
    context.logger?.log(
      `Skipping ${packageName}: no trusted publisher configured for it yet`
    );
    return;
  }
  throw new Error(
    `${packageName} npm publish failed with exit code ${result.status ?? -1}`
  );
}

function isUnconfiguredTrustedPublisher(result) {
  if (process.env.TFOLD_SKIP_MISSING_NATIVE_PUBLISH !== "1") return false;

  const output = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
  return (
    output.includes("npm error code E404") ||
    output.includes("404 Not Found") ||
    output.includes("could not be found or you do not have permission")
  );
}

function binariesDir(cwd) {
  return join(cwd, "dist", "binaries");
}

function releaseDir(cwd) {
  return join(cwd, "dist", "release");
}

function cwdOf(context) {
  return context.cwd ?? process.cwd();
}

function versionOf(context) {
  const version = context.nextRelease?.version;
  if (!version) throw new Error("semantic-release did not provide nextRelease.version");
  return version;
}
