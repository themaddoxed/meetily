#!/usr/bin/env node
/** Public source/commit privacy guard. Matched values are never printed. */
import { execFileSync } from 'node:child_process';
import { lstatSync, readFileSync, realpathSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';

const maxText = 8 * 1024 * 1024;
const genericHomes = new Set(['developer', 'runner', 'Shared', 'user', 'username', '...']);
const homePattern = /\/(?:Users|home)\/([^/\s"'`<>]+)(?=\/|[\s"'`<>]|$)|[A-Za-z]:\\+Users\\+([^\\\s"'`<>]+)(?=\\|[\s"'`<>]|$)/g;
const emailPattern = /(?<![\w.+-])[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
function publicEmail(value) {
  return value === 'noreply@github.com'
    || /@(?:example\.(?:com|org|net|invalid|test)|users\.noreply\.github\.com)$/.test(value);
}

export function inspect(root) {
  root = realpathSync(root);
  const git = (...args) => execFileSync('git', ['-C', root, ...args], { encoding: 'utf8' });
  const findings = [];
  const paths = new Set(git('ls-files', '--cached', '--others', '--exclude-standard', '-z').split('\0').filter(Boolean));
  // Retina asset filenames resemble email addresses. Accept only actual known
  // asset basenames inside a path/quoted filename/Markdown label, never as a
  // general numeric-domain exception or as a Git identity.
  const retinaNames = new Set([...paths].filter((name) => /@(?:2|3)x\.(?:png|jpe?g|webp)$/.test(name)).map((name) => path.basename(name)));
  let files = 0;
  for (const relative of paths) {
    const file = path.resolve(root, relative);
    let stat;
    try {
      stat = lstatSync(file);
    } catch (error) {
      if (error.code === 'ENOENT') continue; // Staged deletion.
      throw error;
    }
    if (stat.isSymbolicLink() || !realpathSync(file).startsWith(root + path.sep)) {
      findings.push(`privacy.symlink: ${relative}:0`);
      continue;
    }
    if (!stat.isFile()) continue;
    const synthetic = /(?:^|\/)tests?\//.test(relative);
    if (!synthetic && /(?:^|\/)\.env(?:$|\.(?!example$))|\.(?:sqlite3?|db|wav|mp3|m4a|pem|p12|pfx|key|gguf|onnx)$/i.test(relative)) {
      findings.push(`privacy.runtime-data: ${relative}:0`);
    }
    let raw = readFileSync(file);
    if (relative.endsWith('.gz')) {
      try {
        raw = gunzipSync(raw, { maxOutputLength: maxText });
      } catch {
        findings.push(`privacy.invalid-gzip: ${relative}:0`);
        continue;
      }
    }
    if (raw.subarray(0, 8000).includes(0)) continue;
    if (raw.length > maxText) {
      findings.push(`privacy.text-limit: ${relative}:0`);
      continue;
    }
    const text = raw.toString('utf8');
    files += 1;
    // Synthetic identities in test source are fixtures; the secret policy and
    // independent secret scan still apply to all source files.
    if (synthetic) continue;
    for (const match of text.matchAll(homePattern)) {
      if (!genericHomes.has(match[1] ?? match[2])) findings.push(`privacy.personal-home: ${relative}:${text.slice(0, match.index).split('\n').length}`);
    }
    for (const match of text.matchAll(emailPattern)) {
      const before = text[match.index - 1] ?? '';
      const after = text[match.index + match[0].length] ?? '';
      const assetReference = retinaNames.has(match[0])
        && /[\/\["'`]/.test(before) && /[\]"'`)]/.test(after);
      if (!publicEmail(match[0]) && !assetReference) findings.push(`privacy.contact-email: ${relative}:${text.slice(0, match.index).split('\n').length}`);
    }
  }
  const commits = git('log', '--all', '--format=%H%x00%an%x00%ae%x00%cn%x00%ce').trim().split('\n');
  for (const line of commits) {
    const [sha, author, email, committer, committerEmail] = line.split('\0');
    for (const [name, address] of [[author, email], [committer, committerEmail]]) {
      if (!publicEmail(address) || (name.includes(' ') && !['Meetily Contributors', 'GitHub Actions'].includes(name))) findings.push(`privacy.commit-identity: ${sha.slice(0, 12)}:0`);
    }
  }
  return { files, commits: commits.length, findings: [...new Set(findings)].sort() };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.length && (args.length !== 2 || args[0] !== '--root')) throw new Error('Usage: check-publication-privacy.mjs [--root repository]');
  const result = inspect(args[1] ?? path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..'));
  for (const finding of result.findings) console.log(`${finding} (matched value redacted)`);
  console.log(`Public privacy: ${result.files} text files, ${result.commits} commits, ${result.findings.length} findings`);
  process.exitCode = result.findings.length ? 1 : 0;
}
