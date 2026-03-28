# xanthos.dev — Portfolio Plan
*Audit date: 2026-03-28 | Build session: pending*

---

## 1. What Exists Today (Audit Summary)

### Pages
| File | Nav Label | Current State |
|------|-----------|--------------|
| `index.html` | Home | Landing page branded as "Learn Rust by converting Processing" — needs complete rewrite as portfolio |
| `story.html` | Ode | Literary content with POS-highlighting JS. Several `[placeholder]` sections still visible. |
| `preview-egg.html` | Curtain | Two-panel hero: Rust code + prose. Functions as an implicit About page. |
| `artClub.html` | artClub | Links to `processing-sketches/index.html` — **broken link, directory does not exist** |
| `tombs.html` | Tombs | Full XR concept doc with live p5.js animation. Elevate to a proper project page. |
| `pipeline.html` | easter egg (hidden) | Stores CATALOG→PORT LLM prompt. Keep as URL-only secret. |
| `xanthos_dreams.html` | not in nav | Full-screen canvas animation (boids → crabs). Best live demo in the repo. |
| `sketch_gallery.html` | not in nav | 3D flip-card gallery UI reading `sketch_catalog.json`. Needs real data to be useful. |

### Tech Stack
- Pure static HTML/CSS/JS. No build tool. No framework.
- p5.js via CDN on `tombs.html` only.
- CSS is inline in every file (no shared stylesheet yet).
- Font: Minion Pro (Adobe paid font) → falls back to Georgia. Fine.
- Color palette: `#D4AF37` (gold), `#CE4F1A` (rust/orange), `#1a1a1a` / `#0f1113` (backgrounds).

### Deploy Status
**Not deployed.** No git remote, no CNAME, no GitHub Pages or Netlify config.

### Issues to Fix Before Launch
1. `artClub.html` has a broken link to `processing-sketches/index.html` — fix or repurpose the page
2. `index.html` has no `<meta name="viewport">` — mobile layout will break
3. `._` macOS metadata files clutter the repo — add patterns to `.gitignore`
4. Several `story.html` sections have placeholder text (`"Add the next paragraphs here"`) — not professional
5. Nav is duplicated verbatim in every HTML file — fragile to maintain

---

## 2. Project Blurbs

### nik3 / CamelBeast — Persian Language Acquisition
**Status: in progress**

Most Persian learning tools treat the language as a vocabulary list. CamelBeast treats it as a living corpus. Designed for adult Farsi acquisition, it ingests a 21,857-article dataset drawn from 7 sources and builds a D3 force-directed graph of how terms actually co-occur in real writing — so a learner can see that كتاب (book) lives near دانشگاه (university) and هزار (thousand) in the way native reading would teach. TTS powered by Chatterbox and Montreal Forced Aligner will add pronunciation anchoring. Built in Rust + TypeScript + Vite with a SQLite backing store. The corpus pipeline is complete; the graph visualization and TTS module are under active development.

*Stack: Rust, TypeScript, Vite, SQLite, Python*
*Repo: github.com/qxaminer (audit before linking — confirm what is public)*

---

### SK8-XR — AR Skateboarding Instruction
**Status: in progress (SMU CRCP6380 capstone, due April 2026)**

Learning a skateboard trick the traditional way means falling repeatedly and hoping someone films you. SK8-XR uses iPhone LiDAR to scan a skate spot in real time, then overlays motion capture guidance directly onto the terrain — so a beginner can see the trajectory of a kickflip as a ghost overlay before attempting it. The same system works for fingerboard techniques at desk scale. Built in Unity with ARKit.

*Stack: Unity, ARKit, Swift*
*Action: Film demo video before April deadline*

---

### Ghost Mouse — Trimodal AI Learning Tool
**Status: concept, patent pending**
**⚠️  NO source code, NO technical details, NO architecture diagrams public — see Section 5**

Software tutorials describe steps in text. Ghost Mouse shows them. A trimodal AI learning tool, it overlays a transparent "ghost cursor" that demonstrates navigation, selection, and interaction in any application — the AI doesn't tell you where to click, it shows you, then fades. The mechanic generalizes across skill levels and software contexts.

*Show: problem statement and product vision only*

---

### Ephemeral Gallery — Site-Specific AR Installation
**Status: complete (SMU CRCP6380)**

The Fly is a bar in Austin. Ephemeral Gallery is a site-specific AR installation that lives inside it. Using Scaniverse photogrammetry scans of the physical space as anchors, seven interactive artworks are embedded in the actual architecture — viewable only through a phone camera, only in that location. The work explores the question of what it means to own an experience that exists only when you're standing in the right place.

*Stack: Unity AR, Scaniverse*
*Action: Record video walkthrough of all 7 functions*

---

### sQribe — Offline Handwriting OCR
**Status: complete (submitted)**

Handwritten notes don't search. sQribe fixes that without requiring the cloud. Point your phone at a page of handwriting, and the app returns structured text — entirely on-device, no data uploaded, no subscription. PaddleOCR handles recognition, FastAPI handles the local API surface. The offline-first constraint was intentional: the use cases most dependent on handwriting (field notes, medical records, personal journals) are also the cases least suited to cloud processing.

*Stack: PaddleOCR, FastAPI, Python*
*Live demo option: Hugging Face Spaces (no server required)*

---

### L00Q — Pastoral Biblical Q&A
**Status: demoed, not publicly hosted**

Concordances and keyword searches treat scripture as a database. L00Q treats it as a conversation partner. A pastoral Q&A tool built on the KJV Bible with RAG retrieval, it responds in natural language and speaks its answers aloud using Kokoro TTS — and it listens, via faster-whisper STT, so the exchange can happen hands-free. Built for anyone who wants to think through a text rather than look something up.

*Stack: KJV RAG, Kokoro TTS, faster-whisper STT*
*Action: Record voice interaction demo video*

---

### QoZ — Resource-Aware Multi-LLM Orchestrator
**Status: concept**

Running every query through a frontier API is expensive. Running everything through a local model is slow and often insufficient. QoZ is a resource-aware multi-LLM orchestrator that routes queries intelligently: a factual lookup goes to a local 3B model, a reasoning task goes to a frontier API, a latency-sensitive response returns from whichever backend responds first. The routing logic is aware of compute budget, task type, and response quality requirements.

*Show: vision document and architecture sketch only. No implementation.*

---

### Xanthos / Nannou — Learn Rust by Converting Processing
**Status: complete (ongoing expansion)**

The origin project. A set of Nannou (Rust creative coding framework) visualizations built by porting Processing sketches — each one a lesson in ownership, borrowing, iterators, and concurrency taught through boids, shaders, and audio rather than exercises. The `xanthos_dreams.html` canvas animation (a crab dreaming of fish swirling into arrays) is a live web port of the same aesthetic.

*Stack: Rust, Nannou, wgpu*
*Live demo: `xanthos_dreams.html` — already deployed*

---

## 3. Target Structure

```
/                          Landing page — hero, project grid, brief about, contact
/work/                     Project index (all projects listed)
/work/nik3/                CamelBeast — Persian acquisition
/work/sk8-xr/              AR skateboarding instruction
/work/ghost-mouse/         Concept only — patent pending
/work/ephemeral-gallery/   AR installation at The Fly
/work/sqribe/              Offline handwriting OCR
/work/l00q/                Pastoral biblical Q&A
/work/qoz/                 Multi-LLM orchestrator concept
/work/xanthos/             Xanthos / Nannou — the teaching project
/demos/                    Live interactive work
/demos/xanthos-dreams/     Canvas animation (currently xanthos_dreams.html)
/demos/tombs/              Tombs of the Prophets concept + p5 scene
/about/                    One page — who you are
```

Nav: **Work · Demos · About**

Three items. No easter eggs in primary nav. Keep `pipeline.html` as a URL-only secret at `/catalog`.

---

## 4. Recommended Stack

**Plain HTML/CSS/JS. No build tool.**

You have active projects in Rust, Unity, and Python. Adding a Node.js build system means a fourth ecosystem to maintain. Portfolio sites are exactly the things that break after 6 months when Node has drifted two major versions and `npm install` fails silently.

The current stack already produces good results. The main maintenance pain (nav duplication across files) is solved by a shared nav snippet injected by a single small `<script>`, not a framework.

**The one exception:** If you build Tombs of the Prophets as a real product, your own pre-plan correctly calls for Astro + A-Frame. Keep that as a separate project repo, not the portfolio.

### Shared nav pattern (zero dependency)
In each page, replace the inline `<nav>` with:
```html
<div id="nav-root"></div>
<script src="/nav.js"></script>
```
`nav.js` writes the nav HTML once. Change nav once, it updates everywhere.

---

## 5. Live Demo Strategy

| Project | Demo Format | Live / Embedded? | Notes |
|---------|-------------|-----------------|-------|
| nik3/CamelBeast | Screenshots + corpus stats chart | No — in progress | No pipeline source, no OSINT methodology |
| SK8-XR | Video demo | No — iOS native | Film before April 2026 deadline |
| **Ghost Mouse** | **Text only — problem + vision** | **No** | **Patent pending. Zero implementation details.** |
| Ephemeral Gallery | Video walkthrough (all 7 functions) | Possibly WebGL build | Record at The Fly |
| sQribe | Demo GIF + before/after screenshots | Maybe — Hugging Face Spaces | No server cost |
| L00Q | Voice demo video | No — local TTS/STT | Record conversation session |
| QoZ | One-page vision doc | No — concept only | Architecture sketch is fine |
| Xanthos/Nannou | `xanthos_dreams.html` — already live | **Yes — link directly** | Best live demo in the repo |
| Tombs of Prophets | `tombs.html` with p5 scene — already live | **Yes — link directly** | |

---

## 6. Hosting Recommendation

**GitHub Pages with custom domain xanthos.dev**

- You're already on GitHub (qxaminer)
- Free, zero maintenance, HTTPS via Let's Encrypt automatic
- Deploy: push to `main` → site updates in ~60 seconds
- Custom domain: add a `CNAME` file containing `xanthos.dev` to the repo root, then set the DNS A records at your registrar

```
# CNAME file contents
xanthos.dev
```

DNS records to add at your registrar (GitHub Pages IPs):
```
A  @  185.199.108.153
A  @  185.199.109.153
A  @  185.199.110.153
A  @  185.199.111.153
CNAME  www  qxaminer.github.io
```

Then in GitHub repo Settings → Pages → set source branch to `main`, custom domain to `xanthos.dev`.

**Why not Netlify:** Good option, slightly better for form handling and preview deploys — neither matters here. One more vendor relationship you don't need.
**Why not Vercel:** React-optimized. Nothing here needs Vercel.

---

## 7. Migration Checklist (ordered by priority)

### Pre-build fixes (do before writing any new pages)
- [ ] Fix `.gitignore` — add `._*` and `._.DS_Store` patterns
- [ ] Add `<meta name="viewport" content="width=device-width, initial-scale=1.0">` to `index.html`
- [ ] Remove or fill placeholder text in `story.html`
- [ ] Create `nav.js` shared nav include

### Structure
- [ ] Rewrite `index.html` as portfolio landing (hero, project grid, brief contact)
- [ ] Create `/work/` index page
- [ ] Create one project page per project (8 total)
- [ ] Move `xanthos_dreams.html` → `/demos/xanthos-dreams/index.html`
- [ ] Move `tombs.html` → `/demos/tombs/index.html` (keep existing URL redirecting or update links)
- [ ] Create `/about/index.html`
- [ ] Repurpose `artClub.html` or remove the broken link

### Content (gather before building)
- [ ] Record SK8-XR demo video (before April 2026)
- [ ] Record Ephemeral Gallery video at The Fly
- [ ] Record L00Q voice interaction demo
- [ ] Make sQribe demo GIF (input photo → output text)
- [ ] Audit github.com/qxaminer — confirm which nik3/CamelBeast repos are public before linking
- [ ] Write Ghost Mouse concept page — problem statement only, no technical content
- [ ] Write QoZ vision doc

### Deploy
- [ ] Add `CNAME` file with `xanthos.dev`
- [ ] Enable GitHub Pages in repo settings (branch: `main`)
- [ ] Set DNS A records at domain registrar
- [ ] Verify HTTPS works after propagation (~24h)

---

## 8. What NOT to Publish

| Category | What to withhold | Why |
|----------|-----------------|-----|
| Ghost Mouse | All source code, architecture, technical mechanic details | Patent pending. Publishing implementation details could compromise IP protection. Show concept and problem only. |
| nik3/CamelBeast pipeline | Source scraping code, specific data source list, OSINT methodology | Could expose methodology to replication or legal scrutiny. Show corpus statistics and outcomes, not the pipeline internals. |
| nik3/CamelBeast | Any article content reproduced verbatim | Copyright. Cite sources, don't reproduce. |
| tinyLM / QoZ | Blind testing data, API key patterns, `data-branches-gemini.json` | Contains actual prompt-response data. May contain PII even with sanitizer. Keep local. |
| Any `.env` or `.env.example` | API keys, even example/template keys | Don't train people to think template keys are safe to commit. |
| `files.zip`, `rabbit_hole.zip`, `._*` files | Binary/archive artifacts | No reason to serve these publicly. Already in `.gitignore` pattern territory. |

---

## 9. Visual Identity Notes (for build session)

Carry these forward unchanged:
- **Colors:** `#D4AF37` (gold), `#CE4F1A` (rust/orange), `#8B3010` (hover-dark), `#1a1a1a` (warm dark), `#0f1113` (cool dark), `#f5efe6` (warm light text), `#c7b8a0` (muted text)
- **Font:** `"Minion Pro", Georgia, serif` for body; `"SF Mono", Monaco, monospace` for code
- **Crab logo animation:** Keep the Iron/Rust crab crossfade — it's the brand
- **Tone:** The site can be literary without being obscure. The story pages earn that register. The project pages should be clear and direct.

---

*Next session: build the new `index.html` first, then the shared nav, then project pages.*
