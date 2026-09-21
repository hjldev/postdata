import { spawnSync } from "node:child_process";
// Tauri checks the CI environment variable to skip Finder AppleScript styling.
const result = spawnSync(
  "npm",
  ["run", "tauri", "build", "--", "--ci", ...process.argv.slice(2)],
  {
    stdio: "inherit",
    env: { ...process.env, CI: "true", TAURI_BUNDLER_DMG_IGNORE_CI: "false" },
  },
);
if (result.error) console.error(result.error.message);
process.exit(result.status ?? 1);
