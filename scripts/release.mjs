import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));

const cliff = spawnSync("git", ["cliff", "--bumped-version"], {
  cwd: root,
  encoding: "utf8",
  stdio: ["ignore", "pipe", "inherit"],
});
if (cliff.status !== 0) process.exit(cliff.status ?? 1);

const version = cliff.stdout.trim().replace(/^v/, "");
const release = spawnSync("cargo", ["release", version, ...process.argv.slice(2)], {
  cwd: `${root}src-tauri`,
  stdio: "inherit",
});
process.exit(release.status ?? 1);
