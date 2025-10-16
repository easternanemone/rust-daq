from playwright.sync_api import sync_playwright, expect

def run(playwright):
    chromium = playwright.chromium
    browser = chromium.connect_over_cdp("http://localhost:9222")
    context = browser.contexts[0]
    page = context.pages()[0]

    # Check if the "Consolidate" toggle is visible
    consolidate_toggle = page.get_by_text("Consolidate")
    expect(consolidate_toggle).to_be_visible()

    # Take a screenshot
    page.screenshot(path="jules-scratch/verification/verification.png")

    browser.close()

with sync_playwright() as playwright:
    run(playwright)