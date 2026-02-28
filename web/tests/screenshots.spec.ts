import { test, expect } from '@playwright/test';
import path from 'path';
import fs from 'fs';

const BASE_URL = 'http://localhost:3000';
const SCREENSHOT_DIR = path.join(__dirname, 'screenshots');

test.beforeAll(() => {
  if (!fs.existsSync(SCREENSHOT_DIR)) {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
  }
});

const ROUTES = [
  { name: 'home', path: '/' },
  { name: 'search', path: '/search?q=senator' },
  { name: 'timing', path: '/timing' },
  { name: 'conduct', path: '/conduct' },
  { name: 'submit', path: '/submit' },
];

for (const route of ROUTES) {
  test(`screenshot: ${route.name}`, async ({ page }) => {
    await page.goto(`${BASE_URL}${route.path}`, {
      waitUntil: 'networkidle',
      timeout: 30000,
    });

    // Wait for content to render
    await page.waitForTimeout(1000);

    // Expect the page to load without error
    const title = await page.title();
    expect(title).toBeTruthy();

    // Take screenshot
    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, `${route.name}.png`),
      fullPage: true,
    });

    console.log(`✓ Screenshot saved: ${route.name}.png`);
  });
}

test('home page has hero text', async ({ page }) => {
  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await expect(page.getByText('Every connection')).toBeVisible();
  await expect(page.getByText('traced to its source')).toBeVisible();
});

test('home page has search form', async ({ page }) => {
  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  const searchInput = page.locator('input[name="q"]');
  await expect(searchInput).toBeVisible();
});

test('search page shows results', async ({ page }) => {
  await page.goto(`${BASE_URL}/search`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(500);
  // Check for filter sidebar
  await expect(page.getByText('Filters')).toBeVisible();
});

test('timing page has table', async ({ page }) => {
  await page.goto(`${BASE_URL}/timing`, { waitUntil: 'networkidle' });
  await expect(page.getByText('Timing Correlations')).toBeVisible();
});

test('conduct page has comparison table', async ({ page }) => {
  await page.goto(`${BASE_URL}/conduct`, { waitUntil: 'networkidle' });
  await expect(page.getByText('Conduct Comparison')).toBeVisible();
});

test('submit page has form with validation', async ({ page }) => {
  await page.goto(`${BASE_URL}/submit`, { waitUntil: 'networkidle' });
  await expect(page.getByText('Submit a Connection')).toBeVisible();

  // Try submitting empty form
  await page.getByRole('button', { name: 'Submit for Review' }).click();

  // Should show validation errors
  await expect(page.getByText('Entity name is required')).toBeVisible();
  await expect(page.getByText(/Source URL is required/)).toBeVisible();
});
