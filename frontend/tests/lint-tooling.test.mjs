import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const frontendDir = fileURLToPath(new URL("..", import.meta.url));
const eslintCli = fileURLToPath(
  new URL("../node_modules/eslint/bin/eslint.js", import.meta.url),
);

const toolingTargets = [
  "scripts/*.js",
  "scripts/*.mjs",
  "next.config.js",
  "tailwind.config.js",
  "tailwind.config.ts",
  "tests/universal-build-config.spec.ts",
  "tests/lib/product-analytics-absence.test.mjs",
  "tests/lib/builtin-ai-models.test.mjs",
  "tests/lib/onboarding-summary-model.test.mjs",
  "tests/lib/transcript-recovery-hook.test.mjs",
  "tests/lib/blocknote-markdown.test.ts",
];

test("non-application tooling and config targets pass ESLint", () => {
  const result = spawnSync(
    process.execPath,
    [eslintCli, ...toolingTargets, "--max-warnings=0"],
    {
      cwd: frontendDir,
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 0, `${result.stdout}${result.stderr}`);
});
