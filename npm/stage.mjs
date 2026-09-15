#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import {
  assertManifestsMatchShimTable,
  fail,
  hostKey,
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
  const host = hostKey();
  const nativePackage = NATIVE_PACKAGES[host];
  if (!nativePackage) fail(`No native package defined for ${host}`);

  const binaryName = process.platform === "win32" ? "tfold.exe" : "tfold";
  return {
    hostKey: host,
    nativePackage,
    binary: resolve(
      valueAfter(args, "--binary") ?? join(repoRoot, "target", "release", binaryName)
    ),
    version: valueAfter(args, "--version") ?? "0.0.0",
    outDir: resolve(
      valueAfter(args, "--out") ?? mkdtempSync(join(tmpdir(), "tfold-stage-"))
    ),
    keep: args.includes("--keep"),
  };
}

function stageNativePackage(options) {
  const target = join(options.outDir, "node_modules", options.nativePackage.packageName);
  const binaryTarget = join(target, options.nativePackage.binaryPath);
  mkdirSync(dirname(binaryTarget), { recursive: true });
  copyFileSync(options.binary, binaryTarget);
  chmodSync(binaryTarget, 0o755);
  writeJson(
    join(target, "package.json"),
    stampedManifest(nativeManifestPath(options.hostKey), options.version, {
      publishable: true,
    })
  );
}

function stageShim(options) {
  cpSync(join(npmRoot, "tfold", "bin"), join(options.outDir, "bin"), { recursive: true });
  copyFileSync(join(repoRoot, "README.md"), join(options.outDir, "README.md"));
  writeJson(
    join(options.outDir, "package.json"),
    stampedManifest(shimManifestPath(), options.version)
  );
}

function smoke(options) {
  const shim = join(options.outDir, "bin", "tfold.js");
  const result = spawnSync(
    process.execPath,
    [shim, repoRoot, "--budget", "400", "--cost"],
    {
      encoding: "utf-8",
    }
  );

  if (result.status !== 0) {
    fail(`Shim exited ${result.status}\n${result.stdout ?? ""}${result.stderr ?? ""}`);
  }
  if (!/^~\d+ tokens · [\d.]+ ms$/m.test(result.stdout)) {
    fail(`Shim output has no token footer:\n${result.stdout}`);
  }
  if (!result.stdout.includes("src/")) {
    fail(`Shim output does not look like a tfold map:\n${result.stdout}`);
  }
  return result.stdout;
}

function assertMissingBinaryIsExplained(options) {
  const orphan = mkdtempSync(join(tmpdir(), "tfold-orphan-"));
  try {
    cpSync(join(options.outDir, "bin"), join(orphan, "bin"), { recursive: true });
    copyFileSync(join(options.outDir, "package.json"), join(orphan, "package.json"));
    const result = spawnSync(process.execPath, [join(orphan, "bin", "tfold.js"), "."], {
      encoding: "utf-8",
    });
    if (result.status === 0) fail("Shim succeeded with no native package installed");
    if (!result.stderr.includes(options.nativePackage.packageName)) {
      fail(`Shim did not name the missing package:\n${result.stderr}`);
    }
    return result.stderr.trim();
  } finally {
    rmSync(orphan, { recursive: true, force: true });
  }
}

function npmRun(args, cwd) {
  return spawnSync("npm", args, {
    cwd,
    encoding: "utf-8",
    shell: process.platform === "win32",
  });
}

function pack(packageDir, destination) {
  const result = npmRun(["pack", "--pack-destination", destination], packageDir);
  if (result.status !== 0)
    fail(`npm pack failed in ${packageDir}\n${result.stderr ?? ""}`);
  return join(destination, result.stdout.trim().split("\n").pop());
}

function assertInstalledShimIsNotShadowed(options) {
  const consumer = mkdtempSync(join(tmpdir(), "tfold-consumer-"));
  try {
    writeJson(join(consumer, "package.json"), {
      name: "tfold-install-check",
      version: "0.0.0",
      private: true,
    });
    const tarballs = [
      pack(
        join(options.outDir, "node_modules", options.nativePackage.packageName),
        consumer
      ),
      pack(options.outDir, consumer),
    ];
    const install = npmRun(["install", "--no-audit", "--no-fund", ...tarballs], consumer);
    if (install.status !== 0) fail(`npm install failed\n${install.stderr ?? ""}`);

    const binaryName = process.platform === "win32" ? "tfold.cmd" : "tfold";
    const installed = join(consumer, "node_modules", ".bin", binaryName);
    if (!existsSync(installed)) fail(`npm install created no ${installed}`);

    const shadowCheck = spawnSync(installed, ["."], {
      cwd: repoRoot,
      encoding: "utf-8",
      env: { ...process.env, TFOLD_BINARY: join(consumer, "definitely-not-a-binary") },
      shell: process.platform === "win32",
    });
    if (!shadowCheck.stderr?.includes("could not run")) {
      fail(
        `node_modules/.bin/tfold is not the shim — a native package is shadowing it.\nstdout: ${shadowCheck.stdout}\nstderr: ${shadowCheck.stderr}`
      );
    }

    const real = spawnSync(installed, [repoRoot, "--budget", "200"], {
      encoding: "utf-8",
      shell: process.platform === "win32",
    });
    if (real.status !== 0)
      fail(`installed shim exited ${real.status}\n${real.stderr ?? ""}`);
    return real.stdout.trim().split("\n").pop();
  } finally {
    rmSync(consumer, { recursive: true, force: true });
  }
}

const options = parseArgs();
if (!existsSync(options.binary)) {
  fail(`Binary not found at ${options.binary}. Build it with: cargo build --release`);
}

try {
  assertManifestsMatchShimTable();
  stageNativePackage(options);
  stageShim(options);
  const mapped = smoke(options);
  const explained = assertMissingBinaryIsExplained(options);
  const installed = assertInstalledShimIsNotShadowed(options);

  process.stdout.write(
    [
      `staged ${options.nativePackage.packageName}@${options.version} in ${options.outDir}`,
      `runtime: ${process.execPath}`,
      `map: ${mapped.trim().split("\n").length} lines, ${mapped.trim().split("\n").pop()}`,
      `missing-binary message: ${explained}`,
      `installed via npm, shim not shadowed: ${installed}`,
      "",
    ].join("\n")
  );
} finally {
  if (!options.keep) rmSync(options.outDir, { recursive: true, force: true });
}
