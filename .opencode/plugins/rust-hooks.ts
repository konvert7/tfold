import { spawnSync } from "node:child_process";

const HOOKS = ["rust-format", "rust-check", "rust-clippy"];

function runHookScript(name: string) {
  return spawnSync("bun", [`.agents/hooks/${name}.ts`, "--opencode"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function reportFailure(name: string, result: ReturnType<typeof runHookScript>) {
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
  process.stderr.write(`${name} failed:\n${output}\n`);
}

export const RustHooksPlugin = async () => ({
  event: async ({ event }: { event: { type: string } }) => {
    if (event.type !== "session.idle") return;

    for (const name of HOOKS) {
      const result = runHookScript(name);
      if ((result.status ?? -1) !== 0) reportFailure(name, result);
    }
  },
});

export default RustHooksPlugin;
