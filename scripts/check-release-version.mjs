import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const json = (path) => JSON.parse(readFileSync(path, 'utf8'));
const version = json('package.json').version;
assert.equal(typeof version, 'string', 'Missing package.json version');
const lock = json('package-lock.json');
const versions = {
  'package-lock.json': lock.version,
  'package-lock.json packages[""]': lock.packages[''].version,
  'src-tauri/tauri.conf.json': json('src-tauri/tauri.conf.json').version,
};
for (const [path, header] of [
  ['src-tauri/Cargo.toml', '[package]'],
  ['src-tauri/Cargo.lock', '[[package]]'],
]) {
  const entry = readFileSync(path, 'utf8')
    .split(header)
    .slice(1)
    .find((section) => /^name\s*=\s*"topotp"\s*$/m.test(section));
  assert.ok(entry, `Missing topotp package in ${path}`);
  versions[path] = /^version\s*=\s*"([^"]+)"\s*$/m.exec(entry)?.[1];
}
for (const [path, value] of Object.entries(versions))
  assert.equal(value, version, `Version mismatch in ${path}`);
console.log(`Versions aligned: ${version}`);
