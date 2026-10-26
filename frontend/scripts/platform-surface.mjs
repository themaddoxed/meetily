#!/usr/bin/env node
/** Bounded macOS source-surface audit. This is not native/package acceptance. */
import { execFileSync } from 'node:child_process';
import { readFileSync, lstatSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse as parseToml } from 'smol-toml';
import { load as loadYaml } from 'js-yaml';
import ts from 'typescript';
import { validateLegacyRuntimeAbsent } from './release-policy.mjs';

const required = ['Cargo.toml', 'llama-helper/Cargo.toml', 'frontend/package.json',
  'frontend/src-tauri/Cargo.toml', 'frontend/src-tauri/tauri.conf.json'];
const policyTools = new Set(['frontend/scripts/platform-surface.mjs',
  'frontend/scripts/release-policy.mjs', 'frontend/scripts/verify-release-candidate.mjs']);
const foreignTarget = /\b[\w-]+-(?:pc-windows|unknown-linux|apple-ios|linux-android|unknown-freebsd)[\w-]*\b/i;
const foreignCondition = /\b(?:win32|windows|linux|android|ios|freebsd)\b/i;
const foreignPath = /(?:\.(?:exe|msi|deb|rpm|appimage|cmd|bat|ps1)$|(?:^|\/)(?:windows|linux)(?:\/|[-.]))/i;
const runtimePath = /^(?:frontend\/src(?:\/|$)|frontend\/src-tauri\/(?:src|resources|binaries)(?:\/|$)|llama-helper\/src\/)/;
const scriptPath = /^(?:scripts\/|frontend\/scripts\/|frontend\/src-tauri\/build\.rs$|llama-helper\/build\.rs$)/;
const currentDoc = /^(?:README\.md|docs\/(?:development|building[^/]*|testing)\.md|\.github\/workflows\/[^/]+\.md)$/;

function exclusion(filePath) {
  if (policyTools.has(filePath)) return 'Audit tool: rules and verifier fixtures are not product conditions.';
  if (/(?:^|\/)(?:tests?|testing|fixtures|vendor|node_modules)\//.test(filePath)
    || /\.(?:test|spec)\.[cm]?[jt]sx?$/.test(filePath)) return 'Tests, fixtures or third-party source.';
  if (/^(?:docs\/(?:fork|prd|superpowers|adr|release)\/|\.superpowers\/)/.test(filePath)) return 'Historical evidence, plans or release reports; not current operator instructions.';
  if (scriptPath.test(filePath) && /\.py$/.test(filePath)) return 'Development Python tool outside product runtime.';
  return null;
}

function rustConditions(content) {
  // Keep strings (cfg values) while dropping line/nested block comments.
  let out = '', depth = 0, quoted = false, escaped = false;
  for (let i = 0; i < content.length; i += 1) {
    const char = content[i], next = content[i + 1];
    if (depth) {
      if (char === '/' && next === '*') { depth += 1; i += 1; }
      else if (char === '*' && next === '/') { depth -= 1; i += 1; }
      else if (char === '\n') out += '\n';
    } else if (!quoted && char === '/' && next === '*') { depth = 1; out += ' '; i += 1; }
    else if (!quoted && char === '/' && next === '/') {
      while (i < content.length && content[i] !== '\n') i += 1;
      out += '\n';
    } else {
      out += char;
      if (char === '"' && !escaped) quoted = !quoted;
      escaped = quoted && char === '\\' && !escaped;
    }
  }
  const conditions = [];
  const start = /(?:#\s*\[\s*cfg(?:_attr)?\s*\(|\bcfg!\s*\()/g;
  for (const match of out.matchAll(start)) {
    let balance = 1, inString = false, escape = false;
    const begin = match.index + match[0].length;
    let end = begin;
    for (; end < out.length && balance; end += 1) {
      const char = out[end];
      if (char === '"' && !escape) inString = !inString;
      if (!inString && char === '(') balance += 1;
      if (!inString && char === ')') balance -= 1;
      escape = inString && char === '\\' && !escape;
    }
    conditions.push(out.slice(begin, end));
  }
  return conditions;
}

function jsConditions(filePath, content) {
  const source = ts.createSourceFile(filePath, content, ts.ScriptTarget.Latest, true);
  const conditions = [];
  if (source.parseDiagnostics.length) throw new Error('Invalid JavaScript/TypeScript syntax.');
  const visit = (node) => {
    if (ts.isConditionalExpression(node)) conditions.push(node.condition.getText(source));
    if (ts.isIfStatement(node) || ts.isWhileStatement(node) || ts.isDoStatement(node)) conditions.push(node.expression.getText(source));
    if (ts.isSwitchStatement(node)) conditions.push(node.getText(source));
    // Comparisons also include assignment of platform-derived boolean flags.
    if (ts.isBinaryExpression(node) && [ts.SyntaxKind.EqualsEqualsToken,
      ts.SyntaxKind.EqualsEqualsEqualsToken, ts.SyntaxKind.ExclamationEqualsToken,
      ts.SyntaxKind.ExclamationEqualsEqualsToken].includes(node.operatorToken.kind)) conditions.push(node.getText(source));
    ts.forEachChild(node, visit);
  };
  visit(source);
  return conditions.filter((condition) => /\bprocess\.platform\b|\b(?:os\.)?platform\s*\(|\b(?:target_os|OSTYPE)\b/.test(condition));
}

export function scanPlatformSurface(sourceFiles) {
  const findings = [], auditedPaths = [], excludedPaths = [];
  const add = (code, filePath, message) => findings.push({ code, path: filePath, message });
  for (const filePath of required) {
    if (!(filePath in sourceFiles)) add('platform.missing-config', filePath, 'Required platform configuration is absent.');
  }
  let packageJson = {}, tauriConfig = {};
  const workflowFiles = {};
  for (const [filePath, content] of Object.entries(sourceFiles)) {
    const reason = exclusion(filePath);
    if (reason) { excludedPaths.push({ path: filePath, reason }); continue; }
    const workflow = /^\.github\/workflows\/[^/]+\.ya?ml$/.test(filePath);
    const cargo = /(?:^|\/)Cargo\.toml$/.test(filePath);
    const cargoConfig = /(?:^|\/)\.cargo\/config(?:\.toml)?$/.test(filePath);
    const tauri = /^frontend\/src-tauri\/tauri(?:\.[^/]+)?\.conf\.json$/.test(filePath);
    const relevant = workflow || cargo || cargoConfig || tauri || runtimePath.test(filePath)
      || scriptPath.test(filePath) || currentDoc.test(filePath) || foreignPath.test(filePath)
      || filePath === 'frontend/package.json' || filePath.startsWith('backend/');
    if (!relevant) { excludedPaths.push({ path: filePath, reason: 'Outside the documented bounded platform surface.' }); continue; }
    auditedPaths.push(filePath);
    if (foreignPath.test(filePath)) add('platform.path', filePath, 'Foreign platform executable, installer or platform-specific path.');
    if (runtimePath.test(filePath) && /\.py[cowz]?$/.test(filePath)) add('legacy.python-runtime', filePath, 'Python file is inside product runtime/resources.');
    try {
      if (cargo || cargoConfig) {
        const config = parseToml(content);
        const buildTargets = config.build?.target === undefined ? []
          : (Array.isArray(config.build.target) ? config.build.target : [config.build.target]);
        for (const target of [...Object.keys(config.target ?? {}), ...buildTargets]) {
          if (typeof target !== 'string' || !(/^(?:aarch64|x86_64)-apple-darwin$/.test(target)
            || (/^cfg\(/.test(target) && /target_os\s*=\s*"macos"/.test(target) && !foreignCondition.test(target) && !/\bnot\s*\(/.test(target)))) {
            add('platform.cargo-target', filePath, `Non-macOS or unresolved Cargo target: ${target}.`);
          }
        }
      }
      if (filePath === 'frontend/package.json' || tauri) {
        const config = JSON.parse(content);
        if (!config || typeof config !== 'object' || Array.isArray(config)) throw new Error('Configuration requires an object.');
        if (filePath === 'frontend/package.json') packageJson = config;
        else {
          if (filePath === 'frontend/src-tauri/tauri.conf.json') tauriConfig = config;
          const targets = config.bundle?.targets;
          if (targets !== undefined && (!Array.isArray(targets) || targets.some((target) => !['app', 'dmg'].includes(target)))) {
            add('platform.bundle', filePath, 'Explicit bundle targets must be app/dmg only.');
          }
        }
      }
      if (workflow) {
        const config = loadYaml(content);
        if (!config || typeof config !== 'object' || Array.isArray(config) || !config.jobs
          || typeof config.jobs !== 'object' || Array.isArray(config.jobs)) throw new Error('Workflow requires a jobs mapping.');
        workflowFiles[filePath] = content;
        for (const [name, job] of Object.entries(config.jobs)) {
          if (!job || typeof job !== 'object' || Array.isArray(job)) throw new Error(`Invalid job ${name}.`);
          if (job.uses) {
            if (!/^\.\/\.github\/workflows\/[^/]+\.ya?ml$/.test(job.uses) || !(job.uses.slice(2) in sourceFiles)) {
              add('platform.workflow-runner', filePath, `Job ${name} uses an unresolved reusable workflow.`);
            }
          } else {
            const expression = typeof job['runs-on'] === 'string' && job['runs-on'].match(/^\$\{\{\s*matrix\.(\w+)\s*\}\}$/);
            const matrix = job.strategy?.matrix;
            const runners = expression ? [...(Array.isArray(matrix?.[expression[1]]) ? matrix[expression[1]] : []),
              ...(Array.isArray(matrix?.include) ? matrix.include.map((item) => item?.[expression[1]]).filter((item) => item !== undefined) : [])]
              : [job['runs-on']];
            if (!runners.length || runners.some((runner) => typeof runner !== 'string' || !/^macos-[\w.-]+$/.test(runner))) {
              add('platform.workflow-runner', filePath, `Job ${name} has a non-macOS or unresolved runner.`);
            }
          }
          const visit = (value) => {
            if (!value || typeof value !== 'object') return;
            for (const [key, child] of Object.entries(value)) {
              if (key === 'if' && typeof child === 'string' && foreignCondition.test(child)) add('platform.workflow-condition', filePath, `Job ${name} has a foreign platform condition.`);
              visit(child);
            }
          };
          visit(job);
        }
      }
    } catch {
      delete workflowFiles[filePath];
      add('platform.parse', filePath, 'Cannot parse platform configuration; inspect the file locally.');
    }
    if (!currentDoc.test(filePath) && foreignTarget.test(content)) add('platform.target', filePath, 'Foreign platform compilation target.');
    if (/\.rs$/.test(filePath) && rustConditions(content).some((condition) => foreignCondition.test(condition))) {
      add('platform.condition', filePath, 'Rust cfg contains a foreign platform condition.');
    }
    if (/\.[cm]?[jt]sx?$/.test(filePath)) {
      try {
        if (jsConditions(filePath, content).some((condition) => foreignCondition.test(condition))) {
          add('platform.condition', filePath, 'JavaScript/TypeScript condition selects a foreign platform.');
        }
      } catch {
        add('platform.parse', filePath, 'Cannot parse JavaScript/TypeScript source.');
      }
    }
    if (scriptPath.test(filePath) && /\.(?:sh|bash)$/.test(filePath)
      && /(?:^|\n)\s*(?:if|elif|case|\[)[^\n]*(?:Linux|Windows|MINGW|MSYS|CYGWIN|linux|win32)/.test(content)) {
      add('platform.condition', filePath, 'Shell condition selects a foreign platform.');
    }
    if (currentDoc.test(filePath) && /(?:^#+\s*[^\n]*(?:Windows|Linux)|(?:works|supports?|supported|platforms?|поддерж)[^\n]*(?:Windows|Linux)|(?:apt(?:-get)?|choco|winget)\s+install)/im.test(content)) {
      add('platform.docs', filePath, 'Current instructions advertise or install a foreign platform.');
    }
  }
  const invalidPaths = new Set(findings.filter((item) => item.code === 'platform.parse').map((item) => item.path));
  findings.push(...validateLegacyRuntimeAbsent({ trackedFiles: new Set(Object.keys(sourceFiles)),
    sourceFiles: Object.fromEntries(Object.entries(sourceFiles).filter(([filePath]) => !invalidPaths.has(filePath))),
    packageJson, tauriConfig, workflowFiles }));
  return { findings, auditedPaths: auditedPaths.sort(), excludedPaths: excludedPaths.sort((a, b) => a.path.localeCompare(b.path)) };
}

export function inspectRepository(root) {
  const files = execFileSync('git', ['-C', root, 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }).split('\0').filter(Boolean);
  const sources = {};
  for (const filePath of new Set(files)) {
    const resolved = path.resolve(root, filePath);
    if (!resolved.startsWith(`${path.resolve(root)}${path.sep}`)) throw new Error(`Path escapes repository: ${filePath}`);
    const stat = lstatSync(resolved);
    // Gitlinks and large binary assets are inventory-only; never follow symlinks.
    if (stat.isSymbolicLink()) throw new Error(`Cannot audit symlink: ${filePath}`);
    sources[filePath] = stat.isFile() && stat.size < 1024 * 1024
      ? readFileSync(resolved, 'utf8') : '';
  }
  return scanPlatformSurface(sources);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  try {
    const index = args.indexOf('--root');
    if (args.some((arg, position) => !['--root', '--json'].includes(arg) && !(index !== -1 && position === index + 1))
      || (index !== -1 && (!args[index + 1] || args[index + 1].startsWith('--')))) throw new Error('Usage: platform-surface.mjs [--root repository] [--json]');
    const root = index === -1 ? path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..') : path.resolve(args[index + 1]);
    const report = inspectRepository(root);
    report.status = report.findings.length ? 'FAIL' : 'PASS';
    if (args.includes('--json')) console.log(JSON.stringify(report, null, 2));
    else {
      console.log(`Platform surface: ${report.status}; ${report.findings.length} findings; ${report.auditedPaths.length} audited paths; ${report.excludedPaths.length} excluded paths.`);
      for (const item of report.findings) console.log(`${item.code}: ${item.path}: ${item.message}`);
    }
    process.exitCode = report.findings.length ? 1 : 0;
  } catch (error) {
    console.error(`Platform surface ERROR: ${error.message}`);
    process.exitCode = 2;
  }
}
