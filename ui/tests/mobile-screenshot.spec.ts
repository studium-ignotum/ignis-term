import { test, devices } from '@playwright/test';

// Use iPhone 12 viewport with Chromium
test.use({
  ...devices['iPhone 12'],
});

test('capture mobile view', async ({ page }) => {
  // Collect console logs
  const logs: string[] = [];
  page.on('console', msg => {
    logs.push(`[${msg.type()}] ${msg.text()}`);
  });

  // Collect errors
  const errors: string[] = [];
  page.on('pageerror', err => {
    errors.push(err.message);
  });

  // Go to login page first
  await page.goto('http://localhost:5173/login');
  await page.waitForTimeout(1000);

  // Enter the session code
  const codeInput = page.locator('input[type="text"], input[placeholder*="code" i], input');
  await codeInput.first().fill('YYJGLR');

  // Click connect button
  const connectBtn = page.locator('button:has-text("Connect"), button[type="submit"]');
  await connectBtn.first().click();

  // Wait for terminal to load
  await page.waitForTimeout(3000);

  // Take screenshot
  await page.screenshot({
    path: 'tests/screenshots/mobile-view.png',
    fullPage: true
  });

  // Print logs
  console.log('\n=== Console Logs ===');
  logs.forEach(log => console.log(log));

  console.log('\n=== Page Errors ===');
  errors.forEach(err => console.log(err));

  // Keep browser open for inspection
  await page.waitForTimeout(60000);
});
