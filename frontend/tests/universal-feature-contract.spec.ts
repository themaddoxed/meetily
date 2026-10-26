import { readFileSync } from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

const read = (relativePath: string) =>
  readFileSync(path.resolve(relativePath), 'utf8');

test('ships the complete Gemma 4 local summary registry', () => {
  const models = read('src-tauri/src/summary/summary_engine/models.rs');

  expect(models).toContain('name: "gemma4:e2b"');
  expect(models).toContain('name: "gemma4:e4b"');
  expect(models).toContain('gemma-4-E2B-it-Q4_K_M.gguf');
  expect(models).toContain('gemma-4-E4B-it-Q4_K_M.gguf');
  expect(models).toContain('90f9618340396838ee7ff5b0ba2da27da62953d3');
  expect(models).toContain('2714b5519c6c3516b1000e7c5e1eba998dfe1fe8');
  expect(models).toContain('"gemma4" => GEMMA4_TEMPLATE');
  expect(models).toContain('<|turn>system');
  expect(models).toContain('<turn|>');
});

test('recommends Gemma 4 for the target Mac memory tiers', () => {
  const commands = read('src-tauri/src/summary/summary_engine/commands.rs');

  expect(commands).toContain('"gemma4:e2b"');
  expect(commands).toContain('"gemma4:e4b"');
  expect(commands).toContain('GEMMA4_E4B_RECOMMENDED_RAM_GB');
  expect(commands).toContain('recommended_summary_model_uses_gemma_e2b_below_24gb_on_macos');
  expect(commands).toContain('recommended_summary_model_uses_gemma_e4b_from_24gb_on_macos');
});

test('lets onboarding select and size every Gemma 4 model', () => {
  const overview = read('src/components/onboarding/steps/SetupOverviewStep.tsx');
  const modelSizes = read('src/lib/onboarding-summary-model.ts');

  expect(overview).toContain("invoke<SummaryModelOption[]>('builtin_ai_list_models')");
  expect(overview).toContain('setSelectedSummaryModel(model.name)');
  expect(overview).toContain('summaryModels.map((model)');
  expect(modelSizes).toContain("'gemma4:e2b': 2963");
  expect(modelSizes).toContain("'gemma4:e4b': 5088");
});

test('keeps Gemma 4 inference stable in the bundled helper', () => {
  const helper = read('../llama-helper/src/main.rs');

  expect(helper).toContain('LlamaLogitBias');
  expect(helper).toContain('build_suppressed_token_biases');
  expect(helper).toContain('LLAMA_GPU_LAYERS');
  expect(helper).toContain('.with_devices(&[])');
});

test('labels the Gemma release consistently as 0.4.1', () => {
  const packageJson = JSON.parse(read('package.json'));
  const tauriConfig = JSON.parse(read('src-tauri/tauri.conf.json'));
  const cargoManifest = read('src-tauri/Cargo.toml');

  expect(packageJson.version).toBe('0.4.1');
  expect(tauriConfig.version).toBe('0.4.1');
  expect(cargoManifest).toContain('version = "0.4.1"');
});

test('shows interrupted meetings on the first relaunch', () => {
  const recovery = read('src/hooks/useTranscriptRecovery.ts');

  expect(recovery).toContain("sessionStorage.getItem('indexeddb_current_meeting_id')");
  expect(recovery).toContain('m.meetingId !== activeMeetingId');
  expect(recovery).not.toContain('secondsAgo');
  expect(recovery).not.toContain('isOldEnough');
});

test('removes the legacy analytics client from the product', () => {
  expect(read('src/app/layout.tsx')).not.toContain('AnalyticsProvider');
  expect(read('src/components/Sidebar/SidebarProvider.tsx')).not.toContain('Analytics.');
});
