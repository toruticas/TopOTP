// Debug .e2e identifier only; native auth/clipboard/file dialogs remain synthetic.
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
function run(command, args, options = {}) {
  const result = spawnSync(command, args, { stdio: 'inherit', ...options });
  if (result.status !== 0) process.exit(result.status ?? 1);
  return result;
}
run('npm', ['run', 'test:e2e:build']);
const prepared = run(
  'python3',
  ['scripts/prepare-vault-helper.py', 'aarch64-apple-darwin'],
  { stdio: 'pipe', encoding: 'utf8' },
);
const helper = JSON.parse(prepared.stdout);
run('/usr/bin/codesign', [
  '--force',
  '--sign',
  helper.certificateSHA1,
  '--identifier',
  'com.rafaelmariano.topotp.e2e',
  '--options',
  'runtime',
  '--timestamp=none',
  resolve('.artifacts/e2e-target/debug/topotp'),
]);
run(resolve('node_modules/.bin/wdio'), ['run', './wdio.conf.mjs'], {
  env: {
    ...process.env,
    TOPOTP_E2E_REAL_STORE: '1',
    TOPOTP_E2E_HELPER: JSON.stringify(helper),
  },
});
