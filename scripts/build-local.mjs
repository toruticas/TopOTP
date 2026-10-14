import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
const args = process.argv.slice(2);
const tauri = resolve('node_modules/.bin/tauri');
let result = spawnSync(tauri, ['build', ...args], { stdio: 'inherit' });
if (result.status !== 0) process.exit(result.status ?? 1);
if (!args.includes('--no-bundle') && !args.includes('--bundles')) {
  const index = args.indexOf('--target');
  const target = index >= 0 ? args[index + 1] : null;
  const profile = args.includes('--debug') ? 'debug' : 'release';
  const app = resolve(
    'src-tauri/target',
    ...(target ? [target] : []),
    profile,
    'bundle/macos/TopOTP.app',
  );
  result = spawnSync('python3', ['scripts/package-dmg.py', app], {
    stdio: 'inherit',
  });
  process.exit(result.status ?? 1);
}
