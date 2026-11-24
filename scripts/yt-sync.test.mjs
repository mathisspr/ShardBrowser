import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const script = fileURLToPath(new URL('../.github/scripts/yt-sync.sh', import.meta.url));
const bash = process.platform === 'win32' ? 'C:/Program Files/Git/bin/bash.exe' : 'bash';
const posix = p => p.replaceAll('\\', '/');

function git(cwd, ...args) {
  const result = spawnSync('git', args, { cwd, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  return result.stdout.trim();
}

for (const scenario of ['unchanged', 'clean merge', 'workflow conflict', 'source conflict']) {
  test(`yt-sync: ${scenario}`, t => {
    const root = mkdtempSync(join(tmpdir(), 'yt-sync-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const upstream = join(root, 'upstream');
    const origin = join(root, 'origin.git');
    const fork = join(root, 'fork');
    mkdirSync(upstream);
    git(upstream, 'init', '-b', 'main');
    git(upstream, 'config', 'user.name', 'Test');
    git(upstream, 'config', 'user.email', 'test@example.invalid');
    git(upstream, 'config', 'core.autocrlf', 'false');
    mkdirSync(join(upstream, '.github/workflows'), { recursive: true });
    writeFileSync(join(upstream, '.github/workflows/release.yml'), 'base workflow\n');
    writeFileSync(join(upstream, 'source.txt'), 'base source\n');
    git(upstream, 'add', '.');
    git(upstream, 'commit', '-m', 'Base');
    git(root, 'clone', '--bare', upstream, origin);
    git(root, '-c', 'core.autocrlf=false', 'clone', origin, fork);
    git(fork, 'checkout', '-b', 'youtube');
    git(fork, 'config', 'user.name', 'Test');
    git(fork, 'config', 'user.email', 'test@example.invalid');
    git(fork, 'config', 'core.autocrlf', 'false');
    writeFileSync(join(fork, '.github/workflows/release.yml'), 'fork workflow\n');
    if (scenario === 'source conflict') writeFileSync(join(fork, 'source.txt'), 'fork source\n');
    git(fork, 'add', '.');
    git(fork, 'commit', '-m', 'Fork customizations');
    git(fork, 'push', 'origin', 'youtube');
    const before = git(fork, 'rev-parse', 'HEAD');
    if (scenario !== 'unchanged') {
      writeFileSync(join(upstream, 'source.txt'), 'upstream source\n');
      if (scenario === 'workflow conflict') {
        writeFileSync(join(upstream, '.github/workflows/release.yml'), 'upstream workflow\n');
        writeFileSync(join(upstream, '.github/workflows/new.yml'), 'upstream only\n');
      }
      git(upstream, 'add', '.');
      git(upstream, 'commit', '-m', 'Upstream update');
    }
    const output = join(root, 'output');
    const summary = join(root, 'summary');
    const result = spawnSync(bash, [posix(resolve(script))], {
      cwd: fork, encoding: 'utf8', env: { ...process.env,
        UPSTREAM_URL: posix(upstream), GITHUB_OUTPUT: posix(output), GITHUB_STEP_SUMMARY: posix(summary) },
    });
    const remote = git(root, '--git-dir', origin, 'rev-parse', 'youtube');
    assert.equal(readFileSync(join(fork, '.github/workflows/release.yml'), 'utf8'), 'fork workflow\n');
    assert.equal(git(fork, 'status', '--porcelain'), '');
    if (scenario === 'source conflict') {
      assert.equal(result.status, 1, result.stdout + result.stderr);
      assert.equal(remote, before, 'A conflicting merge must never be pushed');
      assert.match(readFileSync(summary, 'utf8'), /source\.txt/);
    } else {
      assert.equal(result.status, 0, result.stdout + result.stderr);
      assert.match(readFileSync(output, 'utf8'), scenario === 'unchanged' ? /changed=no/ : /changed=yes/);
      if (scenario === 'unchanged') assert.equal(remote, before);
      else {
        assert.notEqual(remote, before);
        git(fork, 'merge-base', '--is-ancestor', git(upstream, 'rev-parse', 'HEAD'), 'HEAD');
        assert.equal(readFileSync(join(fork, 'source.txt'), 'utf8'), 'upstream source\n');
        assert.equal(git(fork, 'ls-tree', '--name-only', 'HEAD', '.github/workflows/new.yml'), '');
      }
    }
  });
}
