import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const npmRoot = dirname(fileURLToPath(import.meta.url));
export const repoRoot = join(npmRoot, "..");

export const NATIVE_PACKAGES = {
  "darwin-arm64": {
    packageName: "@konvert7/tfold-darwin-arm64",
    binaryPath: "bin/tfold",
    rustTarget: "aarch64-apple-darwin",
  },
  "darwin-x64": {
    packageName: "@konvert7/tfold-darwin-x64",
    binaryPath: "bin/tfold",
    rustTarget: "x86_64-apple-darwin",
  },
  "linux-x64": {
    packageName: "@konvert7/tfold-linux-x64",
    binaryPath: "bin/tfold",
    rustTarget: "x86_64-unknown-linux-gnu",
  },
  "win32-x64": {
    packageName: "@konvert7/tfold-win32-x64",
    binaryPath: "bin/tfold.exe",
    rustTarget: "x86_64-pc-windows-msvc",
  },
};

export function hostKey() {
  return `${process.platform}-${process.arch}`;
}

export function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

export function nativeManifestPath(key) {
  return join(npmRoot, "native", key, "package.json");
}

export function shimManifestPath() {
  return join(npmRoot, "tfold", "package.json");
}

function readManifest(path) {
  try {
    return JSON.parse(readFileSync(path, "utf-8"));
  } catch (error) {
    fail(`Failed to read ${path}: ${error.message}`);
  }
}

export function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

export function stampedManifest(path, version, { publishable = false } = {}) {
  const manifest = readManifest(path);
  manifest.version = version;
  for (const name of Object.keys(manifest.optionalDependencies ?? {})) {
    manifest.optionalDependencies[name] = version;
  }
  if (publishable) delete manifest.private;
  return manifest;
}

export function assertManifestsMatchShimTable() {
  const shim = readFileSync(join(npmRoot, "tfold", "bin", "tfold.js"), "utf-8");
  for (const [key, expected] of Object.entries(NATIVE_PACKAGES)) {
    const manifest = readManifest(nativeManifestPath(key));
    if (manifest.name !== expected.packageName) {
      fail(
        `${key}: manifest name ${manifest.name} does not match ${expected.packageName}`
      );
    }
    if (!manifest.files?.includes(expected.binaryPath)) {
      fail(
        `${key}: manifest files ${JSON.stringify(manifest.files)} does not ship ${expected.binaryPath}`
      );
    }
    if (manifest.bin) {
      fail(
        `${key}: a native package must declare no bin, or it shadows the shim in node_modules/.bin`
      );
    }
    if (!shim.includes(expected.packageName) || !shim.includes(expected.binaryPath)) {
      fail(
        `${key}: the shim does not reference ${expected.packageName}/${expected.binaryPath}`
      );
    }
  }

  const root = readManifest(shimManifestPath());
  const declared = Object.keys(root.optionalDependencies ?? {}).sort();
  const required = Object.values(NATIVE_PACKAGES)
    .map((entry) => entry.packageName)
    .sort();
  if (declared.join(",") !== required.join(",")) {
    fail(
      `Root optionalDependencies ${declared.join(",")} do not match the native packages ${required.join(",")}`
    );
  }
}
