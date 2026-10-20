import { chromium } from "playwright";
import assert from "node:assert/strict";
import { readFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
// Run the server against a disposable workspace. This test creates synthetic saves and recovery copies.
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({
      viewport: { width: 1440, height: 1000 },
    }),
    errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("http://127.0.0.1:7413");
  await page.locator("#busy").waitFor({ state: "hidden" });
  if (
    !(await page.locator("#source option").allTextContents()).includes(
      "starfall-demo",
    )
  ) {
    await page.locator("#demo").click();
    await page.waitForFunction(
      () => document.getElementById("source").value === "starfall-demo",
    );
  }
  await page.locator("#source").selectOption("starfall-demo");
  await page.locator("#capture").click();
  await page.waitForFunction(() => !document.getElementById("verify").disabled);
  assert.equal(
    await page.locator("#verification").textContent(),
    "Not verified",
  );
  await page.locator("#verify").click();
  await page.locator("#details").waitFor({ state: "visible" });
  assert.equal(await page.locator("#verified-count").textContent(), "1");
  assert.match(await page.locator("#files").textContent(), /slot1.sav/);
  await page.locator("#destination").fill("ui-recovery-" + Date.now());
  await page.locator("#restore").click();
  await page.locator("#complete").waitFor({ state: "visible" });
  const path = await page.locator("#restored-path").textContent();
  assert.match(
    await readFile(join(path, "slot1.sav"), "utf8"),
    /fictional_game=Starfall/,
  );
  if (process.env.UI_SCREENSHOTS) {
    await mkdir(process.env.UI_SCREENSHOTS, { recursive: true });
    await page.screenshot({
      path: join(process.env.UI_SCREENSHOTS, "saveshelf-desktop.png"),
      fullPage: true,
    });
  }
  await page.setViewportSize({ width: 390, height: 844 });
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  if (process.env.UI_SCREENSHOTS)
    await page.screenshot({
      path: join(process.env.UI_SCREENSHOTS, "saveshelf-mobile.png"),
      fullPage: true,
    });
  await page.locator("#restore").click();
  await page.locator("#error").waitFor({ state: "visible" });
  assert.match(await page.locator("#error").textContent(), /exist/i);
  assert.match(await readFile(join(path, "slot1.sav"), "utf8"), /level=3/);
  await page.locator("#destination").fill("../outside");
  await page.locator("#restore").click();
  await page.waitForFunction(() =>
    document.getElementById("error").textContent.includes("letters"),
  );
  const denied = await page.request.post("http://127.0.0.1:7413/api/demo", {
    data: {},
  });
  assert.equal(denied.status(), 403);
  assert.deepEqual(errors, []);
  console.log(
    "PASS SaveShelf: synthetic source, snapshot, verification, restore bytes, existing destination protection, traversal rejection, mobile layout, request boundary.",
  );
} finally {
  await browser.close();
}
