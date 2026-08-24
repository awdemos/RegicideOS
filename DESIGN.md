# RegicideOS First-User GPU Homepage — Design Document

## Purpose
A local, self-contained landing page placed on the default RegicideOS account (`regicide`) so a first-time user sees the Vibe Coding Agency GPU rental offering immediately. The page links to the public `vibecodingagency.com/gpus/` catalog and carries RegicideOS attribution through UTM parameters.

## Brand inheritance
This page is an extension of the Vibe Coding Agency marketing site, not a separate brand. It reuses the same dark-tech palette, typography, and shape language found in `/var/home/a/code/vibecodingagency/site/gpus/index.html` and `/var/home/a/code/vibecodingagency/site/regicide-os/index.html`.

## Color tokens
- Background: `#0a0a0a`
- Surface: `#111111`
- Surface raised: `#171717`
- Surface border: `#232323`
- Text primary: `#f4f4f5`
- Text muted: `#a1a1aa`
- Text dim: `#52525b`
- Accent / CTA: `#76b900`
- Accent hover: `#5f9500`
- CTA text: `#0a0a0a`
- Danger: `#ef4444`

## Typography
- Display / headings: Space Grotesk, weights 500–700
- Body / UI: Inter, weights 400–600
- Self-host via Google Fonts CSS link for offline-independence is not required because the page opens in a browser with network access.

## Page anatomy
1. **Fixed header** — Vibe Coding Agency mark + wordmark, plus two primary actions:
   - "GPU rentals" → external `https://vibecodingagency.com/gpus/?utm_source=regicideos&utm_medium=website&utm_campaign=first_user`
   - "UltraWork" → external `https://vibecodingagency.com/gpu-cloud/?utm_source=regicideos&utm_medium=website&utm_campaign=first_user`
   - No hamburger; local page only needs simple navigation.
2. **Hero** — left-aligned headline, 20-word subtext max, two CTAs (primary to GPU catalog, secondary to UltraWork), atmospheric green/blue radial glows, no full-height hero to avoid pushing content below fold.
3. **GPU catalog preview** — three data cards for H200, B200, RTX PRO 6000 showing GPU name, monthly price, vCPU/RAM shape, and a link to the full catalog. Because this is a marketing teaser, we show representative cards rather than a live table.
4. **Workload guidance** — a two-column layout matching the guidance text the user provided, using plain cards with short headings and one-line bullets.
5. **Why Vibe Coding Agency** — three value cards: bare-metal access, no long-term contracts, SSH-ready instances.
6. **Footer** — minimal copyright, support email, and links to the external site.

## Interaction
- All CTA buttons use the accent color with dark text and hover to `accent-hover`.
- Cards lift slightly on hover via Tailwind `hover:-translate-y-1`.
- No JavaScript dependencies beyond Tailwind config; page works offline except for the external links themselves.

## Asset strategy
- No generated images. The visual language comes from color, typography, and glow gradients.
- SVG mark is hand-copied from the Vibe Coding Agency site because it is a brand asset for the same project.

## Desktop integration
- The HTML file is installed as `/home/regicide/homepage.html`.
- A `.desktop` shortcut named `Rent-a-GPU.desktop` is placed on `/home/regicide/Desktop` and opens the local page with the default browser.
- Both are staged by `build-system/catalyst/stages/stage6-finalize.sh` and asserted by `stage7-verify.sh`.
