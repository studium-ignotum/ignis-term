import { test, expect } from '@playwright/test';

const SESSION_CODE = 'Y3W4BD';
const BASE_URL = 'http://localhost:5173';

test.describe('Tab Switching', () => {
  test('should connect and switch tabs', async ({ page }) => {
    // Enable console logging
    page.on('console', msg => {
      console.log(`[Browser ${msg.type()}]`, msg.text());
    });

    // Go to login page
    await page.goto(`${BASE_URL}/login`);

    // Enter session code
    const codeInput = page.locator('input#code');
    await codeInput.fill(SESSION_CODE);

    // Click connect
    await page.click('button[type="submit"]');

    // Wait for redirect to terminal page (connection successful)
    await page.waitForURL(BASE_URL + '/', { timeout: 10000 });
    console.log('Connected to terminal');

    // Wait for tabs to appear
    await page.waitForSelector('.tab-item', { timeout: 10000 });

    // Get all tabs
    const tabs = page.locator('.tab-item');
    const tabCount = await tabs.count();
    console.log(`Found ${tabCount} tabs`);

    // Get terminal content before switch
    const getTerminalContent = async () => {
      // xterm renders to canvas, so we check the buffer via evaluate
      return await page.evaluate(() => {
        const term = (window as any).__terminal;
        if (term) {
          const buffer = term.buffer.active;
          let content = '';
          for (let i = 0; i < buffer.length; i++) {
            const line = buffer.getLine(i);
            if (line) content += line.translateToString(true) + '\n';
          }
          return content.trim();
        }
        return 'NO_TERMINAL';
      });
    };

    // Print tab info
    for (let i = 0; i < tabCount; i++) {
      const tab = tabs.nth(i);
      const title = await tab.locator('.tab-title').textContent();
      const isActive = await tab.evaluate(el => el.classList.contains('active'));
      console.log(`Tab ${i}: "${title}" ${isActive ? '(ACTIVE)' : ''}`);
    }

    if (tabCount >= 2) {
      // Click on first non-active tab
      const activeTabs = page.locator('.tab-item.active');
      const activeCount = await activeTabs.count();
      console.log(`Active tabs count: ${activeCount}`);

      const activeTab = activeTabs.first();
      const activeTitle = await activeTab.locator('.tab-title').textContent();
      console.log(`\nActive tab: "${activeTitle}"`);

      const content1 = await getTerminalContent();
      console.log(`Content before switch (first 200 chars): ${content1.slice(0, 200)}`);

      // Find a non-active tab and click it
      for (let i = 0; i < tabCount; i++) {
        const tab = tabs.nth(i);
        const isActive = await tab.evaluate(el => el.classList.contains('active'));
        if (!isActive) {
          const title = await tab.locator('.tab-title').textContent();
          console.log(`\nSwitching to tab: "${title}"`);
          await tab.click();
          break;
        }
      }

      // Wait a bit for the switch to happen
      await page.waitForTimeout(1000);

      // Check new active tab
      const newActiveTab = page.locator('.tab-item.active');
      const newActiveTitle = await newActiveTab.locator('.tab-title').textContent();
      console.log(`New active tab: "${newActiveTitle}"`);

      const content2 = await getTerminalContent();
      console.log(`Content after switch (first 200 chars): ${content2.slice(0, 200)}`);

      // Compare
      if (content1 === content2) {
        console.log('\n*** WARNING: Terminal content did not change! ***');
      } else {
        console.log('\n*** SUCCESS: Terminal content changed! ***');
      }

      // Switch back
      console.log('\nSwitching back to original tab...');
      for (let i = 0; i < tabCount; i++) {
        const tab = tabs.nth(i);
        const title = await tab.locator('.tab-title').textContent();
        if (title === activeTitle) {
          await tab.click();
          break;
        }
      }

      await page.waitForTimeout(1000);
      const content3 = await getTerminalContent();
      console.log(`Content after switching back (first 200 chars): ${content3.slice(0, 200)}`);
    }

    // Keep browser open for manual inspection
    console.log('\nTest complete. Keeping browser open for 30 seconds...');
    await page.waitForTimeout(30000);
  });
});
