import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const tag = `v${process.env.NEW_VERSION}`;
const args = ["cliff", "--tag", tag];
if (process.env.DRY_RUN === "true") {
  args.push("--unreleased");
} else {
  args.push("--output", "CHANGELOG.md");
}

const { status } = spawnSync("git", args, {
  cwd: fileURLToPath(new URL("..", import.meta.url)),
  stdio: "inherit",
});
process.exit(status ?? 1);
