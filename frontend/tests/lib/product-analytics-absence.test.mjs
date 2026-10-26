import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const frontendRoot = fileURLToPath(new URL('../../', import.meta.url));
const repositoryRoot = path.dirname(frontendRoot);

function textFilesUnder(root) {
  if (!fs.existsSync(root)) return [];

  const entries = fs.readdirSync(root, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) return textFilesUnder(entryPath);
    return /\.(lock|rs|toml|ts|tsx)$/.test(entry.name) ? [entryPath] : [];
  });
}

test('application sources contain no product analytics client, calls, or consent surface', () => {
  const analyticsPaths = [
    path.join(frontendRoot, 'src/lib/analytics.ts'),
    path.join(frontendRoot, 'src/components/AnalyticsProvider.tsx'),
    path.join(frontendRoot, 'src/components/AnalyticsConsentSwitch.tsx'),
    path.join(frontendRoot, 'src/components/AnalyticsDataModal.tsx'),
    path.join(frontendRoot, 'src-tauri/src/analytics'),
  ];

  for (const analyticsPath of analyticsPaths) {
    const exists = fs.existsSync(analyticsPath);
    const isEmptyDirectory = exists && fs.statSync(analyticsPath).isDirectory()
      && fs.readdirSync(analyticsPath).length === 0;
    assert.equal(
      exists && !isEmptyDirectory,
      false,
      `removed analytics path still exists: ${path.relative(repositoryRoot, analyticsPath)}`,
    );
  }

  const sourceFiles = [
    ...textFilesUnder(path.join(frontendRoot, 'src')),
    ...textFilesUnder(path.join(frontendRoot, 'src-tauri', 'src')),
    path.join(frontendRoot, 'src-tauri', 'Cargo.toml'),
    path.join(repositoryRoot, 'Cargo.lock'),
  ];
  const forbidden = [
    /posthog/i,
    /\bAnalytics(?:Provider|ConsentSwitch|DataModal)?\b/,
    /analytics(?:OptedIn|DefaultOff|\.json)/i,
    /(?:init|disable|is)_analytics|track_[a-z_]*analytics/i,
  ];

  const findings = [];
  for (const filePath of sourceFiles) {
    const source = fs.readFileSync(filePath, 'utf8');
    for (const pattern of forbidden) {
      if (pattern.test(source)) {
        findings.push(`${path.relative(repositoryRoot, filePath)} matches ${pattern}`);
      }
    }
  }

  assert.deepEqual(findings, [], findings.join('\n'));
});
