import { runHook } from "./run-hook";

const exitCode = runHook(["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]);
process.exit(exitCode);
