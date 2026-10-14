import { resolve } from 'node:path';
const application = resolve('.artifacts/e2e-target/debug/topotp');
export const config = {
  runner: 'local',
  specs: ['./tests/e2e/**/*.spec.mjs'],
  maxInstances: 1,
  services: [
    [
      '@wdio/tauri-service',
      {
        appBinaryPath: application,
        driverProvider: 'embedded',
        embeddedPort: 4445,
        enableLogForwarding: false,
        autoInstallTauriDriver: false,
        autoDownloadEdgeDriver: false,
      },
    ],
  ],
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application } }],
  framework: 'mocha',
  reporters: ['spec'],
  logLevel: 'error',
  waitforTimeout: 10000,
  connectionRetryTimeout: 30000,
  connectionRetryCount: 1,
  mochaOpts: { timeout: 45000 },
  afterTest: async function (test, context, { passed }) {
    if (!passed) {
      const { browser } = await import('@wdio/globals');
      await browser.saveScreenshot(resolve('.artifacts/e2e/failure.png'));
      console.log(await browser.$('body').getText());
    }
  },
};
