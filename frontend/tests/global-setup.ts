import type { FullConfig } from '@playwright/test';

// Next dev compiles these shared entry points on first request. Finish that
// preparation before starting per-test budgets; each test still gets its own
// context, fixtures and unchanged navigation/assertion timeouts.
export default async function prepareRoutes(config: FullConfig) {
  const baseURL = config.projects[0]?.use.baseURL;
  if (!baseURL) throw new Error('E2E route preparation requires a baseURL');
  for (const route of ['/', '/meeting-details']) {
    const response = await fetch(new URL(route, baseURL), {
      signal: AbortSignal.timeout(180_000),
    });
    if (!response.ok) {
      throw new Error(`E2E route preparation failed for ${route}: HTTP ${response.status}`);
    }
    await response.arrayBuffer();
    console.log(`Prepared E2E route ${route}`);
  }
}
