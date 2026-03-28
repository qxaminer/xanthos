#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

const SOURCE_DIR = process.argv[2];
const OUT_DIR = process.argv[3] || process.cwd();

if (!SOURCE_DIR) {
  console.error("Usage: node build_processing_catalog.js <source_dir> [out_dir]");
  process.exit(1);
}

const SKIP_DIRS = new Set(["libraries", "examples"]);

const readText = (filePath) => fs.readFileSync(filePath, "utf8");

const isSkipped = (dirPath) => {
  const parts = dirPath.split(path.sep);
  return parts.some((p) => SKIP_DIRS.has(p));
};

const walkPdeFiles = (dirPath, out = []) => {
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  for (const entry of entries) {
    const full = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      if (isSkipped(full)) continue;
      walkPdeFiles(full, out);
    } else if (entry.isFile() && entry.name.toLowerCase().endsWith(".pde")) {
      if (isSkipped(full)) continue;
      out.push(full);
    }
  }
  return out;
};

const firstCommentLine = (source) => {
  const lines = source.split(/\r?\n/).slice(0, 20);
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    if (trimmed.startsWith("//")) {
      const text = trimmed.replace(/^\/\/\s?/, "");
      if (text.length < 6) continue;
      if (/^[\*\-_=]+$/.test(text)) continue;
      if (!/[a-z]/.test(text)) continue;
      if (/[();]/.test(text)) continue;
      return text;
    }
    if (trimmed.startsWith("/*")) {
      const text = trimmed.replace(/^\/\*\s?/, "").replace(/\*\/$/, "");
      if (text.length < 6) continue;
      if (/^[\*\-_=]+$/.test(text)) continue;
      if (!/[a-z]/.test(text)) continue;
      if (/[();]/.test(text)) continue;
      return text;
    }
  }
  return "";
};

const detectFeatures = (source) => {
  const features = new Set();
  const libs = new Set();

  if (/P3D|size\s*\([^)]*P3D|box\s*\(|sphere\s*\(|rotateX|rotateY|rotateZ|PVector\s*\(.*?,.*?,/i.test(source)) {
    features.add("3d");
  }
  if (/Minim|Audio|SoundFile|Sound|FFT|Oscillator|AudioPlayer|AudioIn/i.test(source)) {
    features.add("audio");
  }
  if (/loadImage|PImage|image\s*\(|loadShape|PShape|loadFont/i.test(source)) {
    features.add("image");
  }
  if (/mouse|key|touch/i.test(source)) {
    features.add("interaction");
  }
  if (/loadStrings|loadTable|loadXML|loadJSON|loadBytes|Table|JSONObject|JSONArray|String\[\]/i.test(source)) {
    features.add("data");
  }
  const importMatches = source.match(/import\s+([a-zA-Z0-9_\.]+)\s*;/g) || [];
  importMatches.forEach((m) => {
    const name = m.replace(/import\s+|;|\s/g, "");
    if (name) libs.add(name.split(".")[0]);
  });
  if (/PeasyCam|ControlP5|OpenCV|Video|Box2D|Minim|Sound|Gif|G4P/i.test(source)) {
    libs.add("external");
  }
  if (libs.size > 0) features.add("libraries");

  return { features: Array.from(features), libs: Array.from(libs) };
};

const complexityFrom = (lineCount, fileCount, hasLibs) => {
  if (lineCount > 500 || hasLibs) return "complex";
  if (fileCount > 1 || lineCount >= 100) return "medium";
  return "simple";
};

const portDifficultyFrom = (lineCount, hasLibs, features) => {
  if (hasLibs) return "needs_review";
  if (lineCount > 500 || features.includes("audio") || features.includes("3d")) return "moderate";
  return "straightforward";
};

const sketchNameFromDir = (dirPath) => path.basename(dirPath);

const mainFileFrom = (dirPath, files) => {
  const dirName = sketchNameFromDir(dirPath).toLowerCase();
  const match = files.find((f) => path.basename(f, ".pde").toLowerCase() === dirName);
  return match || files[0];
};

const buildCatalog = () => {
  const pdeFiles = walkPdeFiles(SOURCE_DIR);
  const sketchMap = new Map();

  for (const filePath of pdeFiles) {
    const dir = path.dirname(filePath);
    if (!sketchMap.has(dir)) sketchMap.set(dir, []);
    sketchMap.get(dir).push(filePath);
  }

  const sketches = [];
  for (const [dir, files] of sketchMap.entries()) {
    const sources = files.map((f) => readText(f));
    const combined = sources.join("\n");
    const lineCount = sources.reduce((acc, src) => acc + src.split(/\r?\n/).length, 0);
    const { features, libs } = detectFeatures(combined);
    const complexity = complexityFrom(lineCount, files.length, libs.length > 0);
    const portDifficulty = portDifficultyFrom(lineCount, libs.length > 0, features);
    const comment = firstCommentLine(combined);
    const description = comment
      ? comment
      : `Processing sketch with ${features.length ? features.join(", ") : "core drawing primitives"}.`;
    const mainFile = mainFileFrom(dir, files);
    const sketch = {
      name: sketchNameFromDir(dir),
      path: mainFile,
      description,
      features,
      complexity,
      lines: lineCount,
      files: files.map((f) => path.basename(f)),
      port_difficulty: portDifficulty,
      port_notes: libs.length ? `Uses libraries: ${libs.join(", ")}` : ""
    };
    sketches.push(sketch);
  }

  sketches.sort((a, b) => a.name.localeCompare(b.name));
  return {
    scanned_at: new Date().toISOString(),
    source_dir: SOURCE_DIR,
    sketches
  };
};

const catalog = buildCatalog();
const catalogPath = path.join(OUT_DIR, "sketch_catalog.json");
fs.writeFileSync(catalogPath, JSON.stringify(catalog, null, 2));

const galleryPath = path.join(OUT_DIR, "sketch_gallery.html");
fs.writeFileSync(
  galleryPath,
  `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>Sketch Gallery</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: "Minion Pro", Georgia, serif;
      background: #0b0f12;
      color: #e2f0f3;
      min-height: 100vh;
      padding: 30px;
    }
    h1 { font-size: 2.4em; color: #7dd3fc; margin-bottom: 18px; }
    .filters {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      margin-bottom: 22px;
    }
    .filters select, .filters input {
      background: #0f1720;
      color: #e2f0f3;
      border: 1px solid rgba(125, 211, 252, 0.3);
      border-radius: 8px;
      padding: 8px 10px;
      font-size: 0.95em;
    }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
      gap: 20px;
      perspective: 1200px;
    }
    .card {
      position: relative;
      height: 220px;
      transform-style: preserve-3d;
      transition: transform 0.6s ease;
      cursor: pointer;
    }
    .card.auto-rotate:hover {
      transform: rotateY(180deg);
    }
    .card.flipped {
      transform: rotateY(180deg);
    }
    .face {
      position: absolute;
      inset: 0;
      background: rgba(255, 255, 255, 0.04);
      border: 1px solid rgba(94, 234, 212, 0.25);
      border-radius: 16px;
      padding: 16px;
      backface-visibility: hidden;
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    .front h3 { color: #5eead4; font-size: 1.2em; }
    .badge {
      display: inline-block;
      padding: 2px 8px;
      border-radius: 999px;
      font-size: 0.75em;
      border: 1px solid rgba(125, 211, 252, 0.4);
      color: #7dd3fc;
      width: fit-content;
    }
    .back {
      transform: rotateY(180deg);
    }
    .tags { display: flex; flex-wrap: wrap; gap: 6px; }
    .tag {
      font-size: 0.75em;
      padding: 2px 6px;
      border-radius: 8px;
      background: rgba(94, 234, 212, 0.1);
      border: 1px solid rgba(94, 234, 212, 0.25);
      color: #a7f3d0;
    }
    .muted { color: #cbd5f5; font-size: 0.9em; }
  </style>
</head>
<body>
  <h1>Sketch Catalog</h1>
  <div class="filters">
    <select id="complexityFilter">
      <option value="">Complexity: All</option>
      <option value="simple">simple</option>
      <option value="medium">medium</option>
      <option value="complex">complex</option>
    </select>
    <select id="difficultyFilter">
      <option value="">Port difficulty: All</option>
      <option value="straightforward">straightforward</option>
      <option value="moderate">moderate</option>
      <option value="needs_review">needs_review</option>
    </select>
    <input id="featureFilter" placeholder="feature (3d, audio, image...)" />
    <input id="searchFilter" placeholder="search name..." />
  </div>
  <div class="grid" id="grid"></div>

  <script>
    const grid = document.getElementById("grid");
    const complexityFilter = document.getElementById("complexityFilter");
    const difficultyFilter = document.getElementById("difficultyFilter");
    const featureFilter = document.getElementById("featureFilter");
    const searchFilter = document.getElementById("searchFilter");

    const render = (sketches) => {
      grid.innerHTML = "";
      sketches.forEach((s) => {
        const card = document.createElement("div");
        card.className = "card auto-rotate";
        card.innerHTML = \`
          <div class="face front">
            <div class="badge">\${s.complexity}</div>
            <h3>\${s.name}</h3>
            <div class="muted">\${s.description}</div>
          </div>
          <div class="face back">
            <div class="badge">\${s.port_difficulty}</div>
            <div class="tags">\${(s.features || []).map(f => \`<span class="tag">\${f}</span>\`).join("")}</div>
            <div class="muted">files: \${s.files.length} · lines: \${s.lines}</div>
            <div class="muted">\${s.port_notes || ""}</div>
          </div>\`;
        card.addEventListener("click", () => card.classList.toggle("flipped"));
        grid.appendChild(card);
      });
    };

    const applyFilters = (sketches) => {
      const c = complexityFilter.value;
      const d = difficultyFilter.value;
      const f = featureFilter.value.trim().toLowerCase();
      const q = searchFilter.value.trim().toLowerCase();
      return sketches.filter((s) => {
        if (c && s.complexity !== c) return false;
        if (d && s.port_difficulty !== d) return false;
        if (f && !(s.features || []).some(feat => feat.toLowerCase().includes(f))) return false;
        if (q && !s.name.toLowerCase().includes(q)) return false;
        return true;
      });
    };

    fetch("sketch_catalog.json")
      .then((r) => r.json())
      .then((data) => {
        const sketches = data.sketches || [];
        const update = () => render(applyFilters(sketches));
        [complexityFilter, difficultyFilter, featureFilter, searchFilter].forEach((el) => {
          el.addEventListener("input", update);
          el.addEventListener("change", update);
        });
        update();
      });
  </script>
</body>
</html>`
);

console.log("Wrote", catalogPath, "and", galleryPath);
