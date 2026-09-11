import { runHook } from "./run-hook";

const exitCode = runHook(["cargo", "check", "--all-targets"]);
process.exit(exitCode);
