#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  lstatSync,
  readFileSync,
  readlinkSync,
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

function updateField(hash, value) {
  const content = Buffer.isBuffer(value) ? value : Buffer.from(String(value));
  const length = Buffer.allocUnsafe(8);
  length.writeBigUInt64BE(BigInt(content.length));
  hash.update(length);
  hash.update(content);
}

function git(repository, args) {
  const result = spawnSync('git', args, {
    cwd: repository,
    encoding: null,
    maxBuffer: 100 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) {
    const detail = result.error?.message
      ?? result.stderr?.toString('utf8').trim()
      ?? `exit ${result.status}`;
    throw new Error(`git ${args[0]} failed: ${detail}`);
  }
  return result.stdout;
}

function fileContentHash(filePath) {
  const stat = lstatSync(filePath);
  const hash = createHash('sha256');
  if (stat.isSymbolicLink()) {
    updateField(hash, 'symlink');
    updateField(hash, readlinkSync(filePath));
  } else if (stat.isFile()) {
    updateField(hash, 'file');
    updateField(hash, readFileSync(filePath));
  } else {
    throw new Error(`Unsupported untracked source entry: ${filePath}`);
  }
  return hash.digest('hex');
}

export function sourceFingerprint(repository) {
  const commitSha = git(repository, ['rev-parse', 'HEAD']).toString('utf8').trim();
  const trackedDiff = git(repository, [
    'diff',
    '--binary',
    '--no-ext-diff',
    'HEAD',
    '--',
  ]);
  const untrackedOutput = git(repository, [
    'ls-files',
    '--others',
    '--exclude-standard',
    '-z',
  ]);
  const untrackedPaths = untrackedOutput
    .toString('utf8')
    .split('\0')
    .filter(Boolean)
    .sort();

  const hash = createHash('sha256');
  updateField(hash, 'commit');
  updateField(hash, commitSha);
  updateField(hash, 'tracked-diff');
  updateField(hash, trackedDiff);
  for (const relativePath of untrackedPaths) {
    updateField(hash, 'untracked');
    updateField(hash, relativePath);
    updateField(hash, fileContentHash(path.join(repository, relativePath)));
  }
  return hash.digest('hex');
}

function requiredArgument(args, name) {
  const index = args.indexOf(name);
  if (index === -1 || !args[index + 1]) {
    throw new Error(`Missing required argument: ${name}`);
  }
  return args[index + 1];
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.log('Usage: node scripts/source-fingerprint.mjs --repository <path>');
    return;
  }
  const repository = path.resolve(requiredArgument(args, '--repository'));
  console.log(sourceFingerprint(repository));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(`Source fingerprint failed: ${error.message}`);
    process.exitCode = 1;
  }
}
