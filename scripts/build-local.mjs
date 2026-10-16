import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
const args = process.argv.slice(2);
const tauri = resolve('node_modules/.bin/tauri');
const targetRoot = resolve(process.env.CARGO_TARGET_DIR ?? 'src-tauri/target');
const buildEnv = { ...process.env, CARGO_TARGET_DIR: targetRoot };
const buildArgs = [...args];
{
  let identity = process.env.APPLE_SIGNING_IDENTITY;
  if (process.env.TOPOTP_ADHOC_BUILD === '1') identity = '-';
  else if (!identity) {
    const signer = spawnSync('python3', ['scripts/local-signing-identity.py'], {
      encoding: 'utf8',
    });
    if (signer.status !== 0) {
      process.stderr.write(signer.stderr ?? 'Falha na identidade local.\n');
      process.exit(signer.status ?? 1);
    }
    identity = signer.stdout.trim();
  }
  const resources = JSON.parse(
    readFileSync('src-tauri/tauri.conf.json', 'utf8'),
  ).bundle.resources;
  if (process.env.TOPOTP_ADHOC_BUILD !== '1') {
    const targetIndex = args.indexOf('--target');
    const target =
      targetIndex >= 0
        ? args[targetIndex + 1]
        : process.arch === 'arm64'
          ? 'aarch64-apple-darwin'
          : 'x86_64-apple-darwin';
    const prepared = spawnSync(
      'python3',
      ['scripts/prepare-vault-helper.py', target],
      { encoding: 'utf8' },
    );
    if (prepared.status !== 0) {
      process.stderr.write(prepared.stderr);
      process.exit(prepared.status ?? 1);
    }
    const helper = JSON.parse(prepared.stdout);
    if (identity !== helper.certificateSHA1) {
      throw new Error(
        'A identidade do app deve ser a do helper fixo; trocar exige migração explícita.',
      );
    }
    buildEnv.TOPOTP_VAULT_HELPER_CDHASH = helper.cdhash;
    buildEnv.TOPOTP_VAULT_HELPER_CERT_SHA1 = helper.certificateSHA1;
    resources[helper.binary] = 'vault-helper';
    resources[helper.metadata] = 'vault-helper.json';
  } else {
    delete buildEnv.TOPOTP_VAULT_HELPER_CDHASH;
    delete buildEnv.TOPOTP_VAULT_HELPER_CERT_SHA1;
  }
  const config = JSON.stringify({
    bundle: { macOS: { signingIdentity: identity }, resources },
  });
  mkdirSync(targetRoot, { recursive: true });
  writeFileSync(resolve(targetRoot, 'topotp-bundle-config.json'), config);
  const separator = buildArgs.indexOf('--');
  buildArgs.splice(
    separator < 0 ? buildArgs.length : separator,
    0,
    '--config',
    config,
  );
}
let result = spawnSync(tauri, ['build', ...buildArgs], {
  stdio: 'inherit',
  env: buildEnv,
});
if (result.status !== 0) process.exit(result.status ?? 1);
if (!args.includes('--no-bundle') && !args.includes('--bundles')) {
  const index = args.indexOf('--target');
  const target = index >= 0 ? args[index + 1] : null;
  const profile = args.includes('--debug') ? 'debug' : 'release';
  const app = resolve(
    targetRoot,
    ...(target ? [target] : []),
    profile,
    'bundle/macos/TopOTP.app',
  );
  result = spawnSync('python3', ['scripts/package-dmg.py', app], {
    stdio: 'inherit',
  });
  process.exit(result.status ?? 1);
}
