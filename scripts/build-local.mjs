import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
const args = process.argv.slice(2);
const tauri = resolve('node_modules/.bin/tauri');
const targetRoot = resolve(process.env.CARGO_TARGET_DIR ?? 'src-tauri/target');
const buildEnv = { ...process.env, CARGO_TARGET_DIR: targetRoot };
const buildArgs = [...args];
if (!args.includes('--no-bundle')) {
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
  const config = JSON.stringify({
    bundle: { macOS: { signingIdentity: identity } },
  });
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
