// Machine bridge for the Rust integration test. Stdin/stdout contain only the
// synthetic accounts explicitly constructed by that test; no user vault access.
import { verifyInOriginal, originalBackups } from './roundtrip.mjs';
let input = '';
for await (const chunk of process.stdin) input += chunk;
const request = JSON.parse(input);
await verifyInOriginal(request.plain, null, request.expected);
const { ref, entries } = await verifyInOriginal(
  request.encrypted,
  request.password,
  request.expected,
);
const backups = await originalBackups(ref, entries, request.password);
process.stdout.write(JSON.stringify(backups));
