import { chromium } from 'playwright';

async function main() {
  const browser = await chromium.connectOverCDP('http://localhost:9223');
  const contexts = browser.contexts();
  const page = contexts[0].pages()[0];
  
  // Navigate to viewer management
  await page.click('button:has-text("視聴者管理")');
  await page.waitForTimeout(1000);
  
  // Get dropdown options
  const select = page.locator('.broadcaster-selector select');
  const options = await select.locator('option').all();
  
  console.log('Dropdown options:');
  for (const opt of options) {
    const value = await opt.getAttribute('value');
    const text = await opt.textContent();
    console.log(`  value="${value}" text="${text}"`);
  }
  
  await browser.close();
}

main().catch(console.error);
