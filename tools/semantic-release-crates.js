import { spawnSync } from "node:child_process";

export async function verifyConditions(_, context) {
  if (context.options?.dryRun) return;
  if (!registryToken()) {
    throw new Error(
      "CARGO_REGISTRY_TOKEN is unset; crates.io trusted publishing issued no token"
    );
  }
}

export async function publish(pluginConfig = {}, context) {
  const cwd = cwdOf(context);
  const spawn = pluginConfig.spawnSync ?? spawnSync;

  if (context.options?.dryRun) {
    context.logger?.log("Dry run: skipping the crates.io publish");
    return;
  }

  const result = spawn("cargo", ["publish", "--locked"], {
    cwd,
    encoding: "utf-8",
    stdio: ["ignore", "pipe", "pipe"],
  });

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if ((result.status ?? -1) !== 0) {
    throw new Error(`cargo publish failed with exit code ${result.status ?? -1}`);
  }

  context.logger?.log(`Published tfold ${versionOf(context)} to crates.io`);
}

function registryToken() {
  return process.env.CARGO_REGISTRY_TOKEN;
}

function cwdOf(context) {
  return context.cwd ?? process.cwd();
}

function versionOf(context) {
  const version = context.nextRelease?.version;
  if (!version) throw new Error("semantic-release did not provide nextRelease.version");
  return version;
}
