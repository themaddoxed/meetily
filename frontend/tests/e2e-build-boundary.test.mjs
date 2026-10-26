import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const tauriConfig = JSON.parse(
  readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
);

const inspectConfig = String.raw`
const config = require('./next.config.js');
class NormalModuleReplacementPlugin {
  constructor(_pattern, replacement) {
    this.replacement = replacement;
  }
}
const webpackConfig = config.webpack(
  { plugins: [], resolve: {} },
  { isServer: true, webpack: { NormalModuleReplacementPlugin } },
);
console.log(JSON.stringify({
  replacements: webpackConfig.plugins.map((plugin) => plugin.replacement),
}));
`;

function loadConfig(nodeEnv, e2eTesting) {
  return spawnSync(process.execPath, ['-e', inspectConfig], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      NODE_ENV: nodeEnv,
      NEXT_PUBLIC_E2E_TESTING: e2eTesting,
    },
  });
}

test('production rejects the real E2E bootstrap', () => {
  const result = loadConfig('production', '1');

  assert.notEqual(result.status, 0);
  assert.match(
    `${result.stdout}${result.stderr}`,
    /E2E browser mocks cannot be enabled in a production build/,
  );
});

test('normal production replaces the E2E bootstrap with the no-op module', () => {
  const result = loadConfig('production', '0');

  assert.equal(result.status, 0, result.stderr);
  const output = JSON.parse(result.stdout.trim());
  assert.equal(output.replacements.length, 1);
  assert.match(output.replacements[0], /src\/testing\/E2ENoop\.tsx$/);
});

test('development E2E mode keeps the real bootstrap for Playwright', () => {
  const result = loadConfig('development', '1');

  assert.equal(result.status, 0, result.stderr);
  const output = JSON.parse(result.stdout.trim());
  assert.deepEqual(output.replacements, []);
});

test('native application theme synchronization is explicitly permitted', () => {
  const permissions = tauriConfig.app.security.capabilities.flatMap(
    (capability) => capability.permissions,
  );

  assert.ok(permissions.includes('core:app:allow-set-app-theme'));
});

test('native startup appearance is not forced to a static light theme', () => {
  for (const windowConfig of tauriConfig.app.windows) {
    assert.equal('theme' in windowConfig, false);
  }
});
