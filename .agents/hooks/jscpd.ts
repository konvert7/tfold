import { runHook } from "./run-hook";

const exitCode = runHook(["bun", "run", "jscpd", "--silent"]);
if (exitCode === 0) process.exit(0);

process.stderr.write("\n--- jscpd: offending clones ---\n");
runHook(["bun", "run", "jscpd", "--reporters", "console"]);
process.stderr.write(
  "\n--- ACTION ---\n" +
    "Refactor the duplicated code to eliminate it.\n" +
    "DO NOT raise the threshold in .jscpd.json to silence this.\n"
);
process.exit(exitCode);
