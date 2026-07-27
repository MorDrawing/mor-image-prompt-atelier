#!/usr/bin/env node
// Mor Image Prompt Atelier MCP: stdio, zero deps.
// Helps Grok draft, critique, improve, and store image prompts.
//
// Register (Grok):
//   grok mcp add mor-image-prompts -- node /absolute/path/to/mcp/server.mjs
// Self-check:
//   node server.mjs --check

import { randomUUID } from 'node:crypto';
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const VERSION = '0.3.0';
const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');

const DATA_DIR =
  process.env.MOR_PROMPTS_DATA ||
  join(ROOT, 'data');
const LIBRARY_PATH = join(DATA_DIR, 'library.json');
const DESK_PATH = join(DATA_DIR, 'desk.json');
const PACKS_DIR = join(DATA_DIR, 'packs');
const STYLES_PATH = join(DATA_DIR, 'styles.json');
const FLORA_PATH = join(DATA_DIR, 'flora.json');

// ── IO (packs + desk + flat export) ─────────────────────────────────────────

function sanitizePackId(raw) {
  const s = String(raw || 'inbox')
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return s || 'inbox';
}

function ensureData() {
  mkdirSync(DATA_DIR, { recursive: true });
  mkdirSync(PACKS_DIR, { recursive: true });
  if (!existsSync(DESK_PATH)) {
    writeFileSync(
      DESK_PATH,
      JSON.stringify({ version: 1, next_experiment: null, experiment_history: [] }, null, 2) + '\n',
    );
  }
  if (!existsSync(STYLES_PATH)) {
    writeFileSync(
      STYLES_PATH,
      JSON.stringify({ version: 1, styles: [], media: [], lighting: [], composition: [], aspect_hints: {} }, null, 2) + '\n',
    );
  }
  if (!existsSync(FLORA_PATH)) {
    writeFileSync(
      FLORA_PATH,
      JSON.stringify({ version: 1, fragments: [] }, null, 2) + '\n',
    );
  }
  // Bootstrap empty inbox pack
  const inboxMeta = join(PACKS_DIR, 'inbox', 'pack.json');
  if (!existsSync(inboxMeta)) {
    mkdirSync(join(PACKS_DIR, 'inbox', 'prompts'), { recursive: true });
    mkdirSync(join(PACKS_DIR, 'inbox', 'media'), { recursive: true });
    writeFileSync(
      inboxMeta,
      JSON.stringify(
        {
          format: 'mor-prompt-pack',
          version: 1,
          id: 'inbox',
          title: 'Inbox',
          description: 'Default landing pack for new drafts.',
          tags: ['inbox'],
          license: 'UNLICENSE',
        },
        null,
        2,
      ) + '\n',
    );
  }
}

function loadDesk() {
  ensureData();
  if (!existsSync(DESK_PATH)) {
    return { version: 1, next_experiment: null, experiment_history: [] };
  }
  const d = JSON.parse(readFileSync(DESK_PATH, 'utf8'));
  if (!Array.isArray(d.experiment_history)) d.experiment_history = [];
  return d;
}

function saveDesk(desk) {
  ensureData();
  desk.version = desk.version || 1;
  writeFileSync(DESK_PATH, JSON.stringify(desk, null, 2) + '\n');
}

function ensurePackMeta(packId, titleHint) {
  const id = sanitizePackId(packId);
  const dir = join(PACKS_DIR, id);
  mkdirSync(join(dir, 'prompts'), { recursive: true });
  mkdirSync(join(dir, 'media'), { recursive: true });
  const metaPath = join(dir, 'pack.json');
  if (existsSync(metaPath)) {
    return JSON.parse(readFileSync(metaPath, 'utf8'));
  }
  const title =
    titleHint ||
    id
      .split(/[-_]/)
      .filter(Boolean)
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(' ');
  const meta = {
    format: 'mor-prompt-pack',
    version: 1,
    id,
    title,
    description: `Prompt pack: ${id}`,
    tags: [id],
    license: 'UNLICENSE',
  };
  writeFileSync(metaPath, JSON.stringify(meta, null, 2) + '\n');
  return meta;
}

function inferSubjectClass(p) {
  if (p.subject_class && String(p.subject_class).trim()) {
    return String(p.subject_class).trim().toLowerCase();
  }
  const blob = `${(p.tags || []).join(' ')} ${p.title || ''} ${p.skeleton?.subject || ''}`.toLowerCase();
  if (/animal|pitbull|dog|cat|bird/.test(blob)) return 'animal';
  if (/poster|mucha|banner/.test(blob)) return 'poster';
  if (/professor|wordsmith|scholar|character|anime|person/.test(blob)) return 'character';
  if (/street|atelier|landscape|interior|scene/.test(blob)) return 'scene';
  return 'other';
}

function loadLibrary() {
  ensureData();
  const desk = loadDesk();
  const lib = {
    version: 3,
    next_experiment: desk.next_experiment || null,
    experiment_history: desk.experiment_history || [],
    prompts: [],
    packs: {},
  };

  let packDirs = [];
  try {
    packDirs = readdirSync(PACKS_DIR).filter((name) => {
      try {
        return statSync(join(PACKS_DIR, name)).isDirectory();
      } catch {
        return false;
      }
    });
  } catch {
    packDirs = [];
  }

  // Legacy: only library.json, no packs yet
  if (!packDirs.length && existsSync(LIBRARY_PATH)) {
    const flat = JSON.parse(readFileSync(LIBRARY_PATH, 'utf8'));
    lib.prompts = Array.isArray(flat.prompts) ? flat.prompts : [];
    if (!lib.next_experiment && flat.next_experiment) lib.next_experiment = flat.next_experiment;
    if (!lib.experiment_history?.length && Array.isArray(flat.experiment_history)) {
      lib.experiment_history = flat.experiment_history;
    }
    for (const p of lib.prompts) {
      if (!p.pack_id) p.pack_id = 'inbox';
      p.pack_id = sanitizePackId(p.pack_id);
      p.subject_class = inferSubjectClass(p);
    }
    saveLibrary(lib);
    return loadLibrary();
  }

  for (const packId of packDirs) {
    if (packId.startsWith('.')) continue;
    const meta = ensurePackMeta(packId);
    lib.packs[packId] = meta;
    const promptsDir = join(PACKS_DIR, packId, 'prompts');
    if (!existsSync(promptsDir)) continue;
    for (const file of readdirSync(promptsDir)) {
      if (!file.endsWith('.json')) continue;
      const raw = JSON.parse(readFileSync(join(promptsDir, file), 'utf8'));
      raw.pack_id = sanitizePackId(raw.pack_id || packId);
      raw.subject_class = inferSubjectClass(raw);
      lib.prompts.push(raw);
    }
  }

  lib.prompts.sort((a, b) => String(b.updated_at || '').localeCompare(String(a.updated_at || '')));
  return lib;
}

function reworkIds(lib) {
  return (lib.prompts || []).filter((p) => p.needs_rework).map((p) => p.id);
}

function saveLibrary(lib) {
  ensureData();
  lib.version = 3;
  delete lib.rework_queue;

  saveDesk({
    version: 1,
    next_experiment: lib.next_experiment || null,
    experiment_history: lib.experiment_history || [],
  });

  const keep = new Set();
  for (const p of lib.prompts || []) {
    p.pack_id = sanitizePackId(p.pack_id || 'inbox');
    p.subject_class = inferSubjectClass(p);
    ensurePackMeta(p.pack_id);
    const path = join(PACKS_DIR, p.pack_id, 'prompts', `${p.id}.json`);
    writeFileSync(path, JSON.stringify(p, null, 2) + '\n');
    keep.add(path);
  }

  // GC removed prompt files
  try {
    for (const packId of readdirSync(PACKS_DIR)) {
      const promptsDir = join(PACKS_DIR, packId, 'prompts');
      if (!existsSync(promptsDir)) continue;
      for (const file of readdirSync(promptsDir)) {
        if (!file.endsWith('.json')) continue;
        const path = join(promptsDir, file);
        if (!keep.has(path)) {
          try {
            rmSync(path);
          } catch {
            /* ignore */
          }
        }
      }
    }
  } catch {
    /* ignore */
  }

  // Flat export for grep / older tooling
  const exportLib = {
    version: 3,
    next_experiment: lib.next_experiment || null,
    experiment_history: lib.experiment_history || [],
    prompts: lib.prompts || [],
  };
  writeFileSync(LIBRARY_PATH, JSON.stringify(exportLib, null, 2) + '\n');
}

function listPacks() {
  const lib = loadLibrary();
  const packs = {};
  for (const p of lib.prompts || []) {
    const id = sanitizePackId(p.pack_id || 'inbox');
    if (!packs[id]) {
      packs[id] = {
        id,
        title: lib.packs?.[id]?.title || id,
        tags: lib.packs?.[id]?.tags || [],
        count: 0,
        hot: 0,
        rework: 0,
      };
    }
    packs[id].count += 1;
    if ((p.storage || 'hot') === 'hot') packs[id].hot += 1;
    if (p.needs_rework) packs[id].rework += 1;
  }
  // include empty known packs
  for (const [id, meta] of Object.entries(lib.packs || {})) {
    if (!packs[id]) {
      packs[id] = {
        id,
        title: meta.title || id,
        tags: meta.tags || [],
        count: 0,
        hot: 0,
        rework: 0,
      };
    }
  }
  return {
    count: Object.keys(packs).length,
    packs: Object.values(packs).sort((a, b) => b.count - a.count || a.id.localeCompare(b.id)),
    data_layout: {
      packs: PACKS_DIR,
      desk: DESK_PATH,
      export: LIBRARY_PATH,
    },
  };
}

function loadStyles() {
  ensureData();
  return JSON.parse(readFileSync(STYLES_PATH, 'utf8'));
}

function loadFlora() {
  ensureData();
  return JSON.parse(readFileSync(FLORA_PATH, 'utf8'));
}

function saveFlora(flora) {
  ensureData();
  writeFileSync(FLORA_PATH, JSON.stringify(flora, null, 2) + '\n');
}

function nowIso() {
  return new Date().toISOString();
}

function clauseSet(prompt) {
  return new Set(
    String(prompt || '')
      .split(/[,;]+/)
      .map((s) => s.trim().toLowerCase().replace(/\s+/g, ' '))
      .filter((s) => s.length > 3),
  );
}

function jaccard(a, b) {
  if (!a.size || !b.size) return 0;
  let inter = 0;
  for (const x of a) if (b.has(x)) inter += 1;
  const union = new Set([...a, ...b]).size;
  return union ? inter / union : 0;
}

function findCousins(lib, prompt, excludeId) {
  const needle = clauseSet(prompt);
  const hits = [];
  for (const p of lib.prompts || []) {
    if (excludeId && p.id === excludeId) continue;
    const sim = jaccard(needle, clauseSet(p.prompt));
    if (sim >= 0.55) hits.push({ id: p.id, title: p.title, similarity: Math.round(sim * 100) / 100 });
  }
  hits.sort((a, b) => b.similarity - a.similarity);
  return hits;
}

function pickFlora(flora, { styleId, slot, pool, max = 1, already = [] }) {
  const alreadyLower = already.map((t) => String(t).toLowerCase());
  let candidates = (flora.fragments || []).filter((f) => f.slot === slot);
  if (pool && pool !== 'any') {
    candidates = candidates.filter((f) => f.pool === pool || !f.pool);
  }
  if (styleId) {
    candidates = candidates.filter(
      (f) => !f.style_affinity?.length || f.style_affinity.includes(styleId),
    );
  }
  candidates = candidates.filter((f) => !alreadyLower.includes(String(f.text).toLowerCase()));
  candidates.sort((a, b) => (b.weight || 0) - (a.weight || 0));
  return candidates.slice(0, max);
}

function bumpFlora(flora, fragmentIds, delta) {
  for (const id of fragmentIds || []) {
    const f = (flora.fragments || []).find((x) => x.id === id);
    if (f) f.weight = Math.max(1, Math.min(50, (f.weight || 1) + delta));
  }
}

// ── Prompt craft (deterministic structure for Grok) ─────────────────────────

const SLOTS = [
  { id: 'subject', label: 'Subject', weight: 30, hints: ['who/what is pictured', 'count of figures', 'distinctive features'] },
  { id: 'action', label: 'Action / pose', weight: 10, hints: ['what they are doing', 'gesture', 'expression'] },
  { id: 'setting', label: 'Setting', weight: 15, hints: ['place', 'time of day', 'atmosphere'] },
  { id: 'style', label: 'Style / artist', weight: 15, hints: ['named style or artist', 'genre', 'era'] },
  { id: 'medium', label: 'Medium / texture', weight: 10, hints: ['pen and ink', 'oil', 'manga texture', 'film grain'] },
  { id: 'composition', label: 'Composition', weight: 10, hints: ['crop', 'camera angle', 'layout', 'poster frame'] },
  { id: 'lighting', label: 'Lighting / mood', weight: 10, hints: ['light source', 'contrast', 'mood words'] },
];

const NEGATIVE_PATTERNS = [
  /\bno\s+\w+/gi,
  /\bwithout\s+\w+/gi,
  /\bavoid\s+\w+/gi,
  /\bnot\s+\w+/gi,
  /\bdon't\b/gi,
  /\bdo not\b/gi,
];

const STYLE_MARKERS = {
  pc98: /\b(pc-?98|stippled|photocopied manga|halftone)\b/i,
  'dark-academia': /\b(dark academia|scholarly|cobblestone|leather-bound)\b/i,
  mucha: /\b(mucha|art nouveau|ornate frame)\b/i,
  'pen-ink': /\b(pen and ink|cross-hatch|ink wash|linework)\b/i,
  noir: /\b(noir|streetlamp|wet asphalt|rain-slick)\b/i,
  atelier: /\b(atelier|typewriter|manuscript)\b/i,
  'ukiyo-e': /\bukiyo-?e|woodblock\b/i,
  'oil-baroque': /\b(baroque|chiaroscuro|impasto|rembrandt)\b/i,
};

const MEDIUM_MARKERS = [
  /pen and ink/i,
  /photocopied manga/i,
  /oil on canvas/i,
  /watercolor/i,
  /charcoal/i,
  /screen print/i,
  /woodblock/i,
  /film grain/i,
  /digital illustration/i,
  /linework/i,
];

const LIGHTING_MARKERS = [
  /candlelight/i,
  /moonlit/i,
  /streetlamp/i,
  /neon/i,
  /golden hour/i,
  /overcast/i,
  /chiaroscuro/i,
  /soft (key )?light/i,
  /harsh .* light/i,
  /rim light/i,
  /shadows/i,
];

const COMPOSITION_MARKERS = [
  /portrait/i,
  /wide (establishing )?shot/i,
  /three-quarter/i,
  /low angle/i,
  /birds-?eye/i,
  /rule of thirds/i,
  /poster/i,
  /centered/i,
  /tight crop/i,
  /composition/i,
  /frame/i,
];

function splitClauses(prompt) {
  return String(prompt || '')
    .split(/[,;]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function detectSlots(prompt) {
  const text = String(prompt || '');
  const lower = text.toLowerCase();
  const clauses = splitClauses(text);

  const hasSubject = clauses.length > 0 && clauses[0].split(/\s+/).length >= 2;
  const hasAction = /\b(gathered|brooding|standing|sitting|walking|holding|looking|reading|writing|posing|dancing|running)\b/i.test(text);
  const hasSetting = /\b(street|atelier|room|forest|city|interior|night|library|studio|landscape|background)\b/i.test(text);
  const hasStyle = Object.values(STYLE_MARKERS).some((re) => re.test(text));
  const hasMedium = MEDIUM_MARKERS.some((re) => re.test(text));
  const hasComposition = COMPOSITION_MARKERS.some((re) => re.test(text));
  const hasLighting = LIGHTING_MARKERS.some((re) => re.test(text));

  const found = {
    subject: hasSubject,
    action: hasAction,
    setting: hasSetting,
    style: hasStyle,
    medium: hasMedium,
    composition: hasComposition,
    lighting: hasLighting,
  };

  const negatives = [];
  for (const re of NEGATIVE_PATTERNS) {
    const m = text.match(re);
    if (m) negatives.push(...m);
  }

  let score = 0;
  for (const slot of SLOTS) {
    if (found[slot.id]) score += slot.weight;
  }
  // word-count quality (not too short, not keyword-stuffed)
  const words = lower.split(/\s+/).filter(Boolean).length;
  if (words < 8) score = Math.min(score, 35);
  if (words > 12 && words < 80) score += 5;
  if (negatives.length) score = Math.max(0, score - negatives.length * 5);

  return {
    found,
    score: Math.min(100, score),
    word_count: words,
    clause_count: clauses.length,
    negatives,
    clauses,
  };
}

function matchedStyles(prompt, stylesData) {
  const hits = [];
  for (const style of stylesData.styles || []) {
    const re = STYLE_MARKERS[style.id];
    if (re && re.test(prompt)) hits.push(style.id);
    else if ((style.phrases || []).some((p) => prompt.toLowerCase().includes(p.toLowerCase()))) {
      hits.push(style.id);
    }
  }
  return [...new Set(hits)];
}

function critiquePrompt(prompt) {
  const stylesData = loadStyles();
  const det = detectSlots(prompt);
  const missing = SLOTS.filter((s) => !det.found[s.id]).map((s) => ({
    slot: s.id,
    label: s.label,
    hints: s.hints,
  }));
  const present = SLOTS.filter((s) => det.found[s.id]).map((s) => s.id);
  const styles = matchedStyles(prompt, stylesData);

  const recommendations = [];
  if (!det.found.subject) recommendations.push('Lead with a concrete subject (who/what, count, distinctive traits).');
  if (!det.found.setting) recommendations.push('Name a setting or background so the scene has a place.');
  if (!det.found.style && !det.found.medium) recommendations.push('Add a style or medium (e.g. pen and ink, PC-98, Mucha).');
  if (!det.found.lighting) recommendations.push('Add one lighting/mood cue (candlelight, fog, stippled shadows).');
  if (!det.found.composition) recommendations.push('Optionally pin composition (portrait crop, poster frame, wide shot).');
  if (det.negatives.length) recommendations.push('Rewrite negatives as positives (state what to include, not exclude).');
  if (det.word_count < 12) recommendations.push('Expand slightly. Aim for 2-5 natural sentences or a tight clause list.');
  if (det.word_count > 90) recommendations.push('Trim: one coherent scene, drop keyword spam.');

  let tier = 'C';
  if (det.score >= 85) tier = 'SS';
  else if (det.score >= 70) tier = 'S';
  else if (det.score >= 55) tier = 'A';
  else if (det.score >= 40) tier = 'B';

  return {
    prompt,
    score: det.score,
    tier,
    present_slots: present,
    missing_slots: missing,
    detected_styles: styles,
    negatives_found: det.negatives,
    word_count: det.word_count,
    recommendations,
    craft_order: 'subject > action/pose > setting > style > medium > composition > lighting/mood',
  };
}

function stylePhrases(styleId, stylesData) {
  const s = (stylesData.styles || []).find((x) => x.id === styleId);
  return s?.phrases || [];
}

function improvePrompt(args) {
  const prompt = String(args.prompt || '').trim();
  if (!prompt) throw new Error('prompt is required');

  const stylesData = loadStyles();
  const critique = critiquePrompt(prompt);
  const goalStyle = args.style || critique.detected_styles[0] || null;
  const aspect = args.aspect_ratio || null;
  const intensity = args.intensity || 'balanced'; // lean | balanced | rich

  const clauses = splitClauses(prompt).filter((c) => {
    // drop pure negatives
    return !NEGATIVE_PATTERNS.some((re) => {
      re.lastIndex = 0;
      return re.test(c) && c.split(/\s+/).length <= 4;
    });
  });

  const additions = [];
  if (!critique.present_slots.includes('medium')) {
    const fromStyle = goalStyle ? stylePhrases(goalStyle, stylesData) : [];
    const mediumish = fromStyle.find((p) => /ink|texture|linework|grain|oil|print/i.test(p));
    additions.push(mediumish || stylesData.media?.[0] || '');
  }
  if (!critique.present_slots.includes('lighting')) {
    if (goalStyle === 'pc98') additions.push('stippled shadows');
    else if (goalStyle === 'noir' || goalStyle === 'dark-academia') additions.push('candlelight and long shadows');
    else additions.push(stylesData.lighting?.[2] || stylesData.lighting?.[0] || '');
  }
  if (!critique.present_slots.includes('composition') && intensity !== 'lean') {
    if (goalStyle === 'mucha') additions.push('art nouveau poster composition, ornate frame');
    else additions.push(stylesData.composition?.[0] || '');
  }
  if (goalStyle && !critique.detected_styles.includes(goalStyle)) {
    const phrases = stylePhrases(goalStyle, stylesData);
    if (phrases[0]) additions.push(phrases[0]);
    if (intensity === 'rich' && phrases[1]) additions.push(phrases[1]);
  }
  if (args.extra) additions.push(String(args.extra));

  // Dedup (case-insensitive)
  const seen = new Set();
  const merged = [];
  for (const c of [...clauses, ...additions]) {
    const key = c.toLowerCase().replace(/\s+/g, ' ').trim();
    if (!key || seen.has(key)) continue;
    // skip near-duplicates
    let near = false;
    for (const s of seen) {
      if (s.includes(key) || key.includes(s)) {
        near = true;
        break;
      }
    }
    if (near) continue;
    seen.add(key);
    merged.push(c.trim());
  }

  const improved = merged.join(', ');
  const prose = toProse(merged, goalStyle);

  return {
    original: prompt,
    improved,
    prose_variant: prose,
    critique,
    applied_style: goalStyle,
    aspect_ratio_hint: aspect
      ? { aspect_ratio: aspect, use: stylesData.aspect_hints?.[aspect] || null }
      : null,
    notes_for_agent: [
      'Prefer improved for tag-style models; prose_variant for Imagine/Grok image tools.',
      'Imagine: front-load subject, one scene, no negatives. Save keepers with save_prompt.',
    ],
  };
}

function toProse(clauses, styleId) {
  if (!clauses.length) return '';
  const head = clauses[0];
  const rest = clauses.slice(1);
  const mid = rest.slice(0, Math.ceil(rest.length / 2)).join(', ');
  const tail = rest.slice(Math.ceil(rest.length / 2)).join(', ');
  let out = head.charAt(0).toUpperCase() + head.slice(1);
  if (!/[.!?]$/.test(out)) out += '.';
  if (mid) out += ` ${mid.charAt(0).toUpperCase() + mid.slice(1)}.`;
  if (tail) out += ` Rendered with ${tail}.`;
  if (styleId === 'pc98' && !/black background/i.test(out)) {
    out += ' Black background, delicate high-contrast linework.';
  }
  return out.replace(/\s+/g, ' ').trim();
}

function buildPrompt(args) {
  const stylesData = loadStyles();
  const flora = loadFlora();
  const parts = [];
  const usedFlora = [];
  if (args.subject) parts.push(String(args.subject).trim());
  if (args.action) parts.push(String(args.action).trim());
  if (args.setting) parts.push(String(args.setting).trim());

  const styleId = args.style || null;
  if (styleId) {
    const phrases = stylePhrases(styleId, stylesData);
    if (phrases.length) parts.push(phrases[0]);
    else parts.push(styleId);
  } else if (args.style_phrase) {
    parts.push(String(args.style_phrase).trim());
  }

  if (args.medium) parts.push(String(args.medium).trim());
  else {
    for (const f of pickFlora(flora, { styleId, slot: 'medium', max: 1, already: parts })) {
      parts.push(f.text);
      usedFlora.push(f.id);
    }
  }

  if (args.composition) parts.push(String(args.composition).trim());

  if (args.lighting) parts.push(String(args.lighting).trim());
  else {
    for (const f of pickFlora(flora, { styleId, slot: 'lighting', max: 1, already: parts })) {
      parts.push(f.text);
      usedFlora.push(f.id);
    }
  }

  if (args.extra) parts.push(String(args.extra).trim());

  const floraCapped = usedFlora.slice(0, 2);
  const clause = parts.filter(Boolean).join(', ');
  return {
    prompt: clause,
    prose_variant: toProse(parts.filter(Boolean), styleId),
    critique: critiquePrompt(clause),
    flora_used: floraCapped,
  };
}

function varyPrompt(args) {
  const prompt = String(args.prompt || '').trim();
  if (!prompt) throw new Error('prompt is required');
  const n = Math.min(6, Math.max(1, Number(args.count) || 3));
  const stylesData = loadStyles();
  const styles = stylesData.styles || [];
  const lighting = stylesData.lighting || [];
  const media = stylesData.media || [];
  const base = splitClauses(prompt);
  // strip trailing style-ish tails loosely: keep first 2-4 clauses as core
  const core = base.slice(0, Math.min(4, base.length));

  const variations = [];
  for (let i = 0; i < n; i++) {
    const style = styles[i % styles.length];
    const lit = lighting[i % lighting.length];
    const med = media[i % media.length];
    const clauses = [...core, style.phrases[0], med, lit].filter(Boolean);
    const seen = new Set();
    const uniq = clauses.filter((c) => {
      const k = c.toLowerCase();
      if (seen.has(k)) return false;
      seen.add(k);
      return true;
    });
    variations.push({
      id: `var-${i + 1}`,
      style: style.id,
      prompt: uniq.join(', '),
      prose_variant: toProse(uniq, style.id),
    });
  }
  return { original: prompt, variations };
}

// ── Library CRUD ────────────────────────────────────────────────────────────

function listLibrary(args = {}) {
  const lib = loadLibrary();
  let items = lib.prompts || [];
  if (args.tag) {
    const t = String(args.tag).toLowerCase();
    items = items.filter((p) => (p.tags || []).some((x) => x.toLowerCase() === t));
  }
  if (args.q) {
    const q = String(args.q).toLowerCase();
    items = items.filter(
      (p) =>
        (p.title || '').toLowerCase().includes(q) ||
        (p.prompt || '').toLowerCase().includes(q) ||
        (p.notes || '').toLowerCase().includes(q) ||
        (p.pack_id || '').toLowerCase().includes(q) ||
        (p.subject_class || '').toLowerCase().includes(q) ||
        (p.tags || []).some((x) => x.toLowerCase().includes(q)),
    );
  }
  if (args.tier) {
    items = items.filter((p) => (p.tier || '').toUpperCase() === String(args.tier).toUpperCase());
  }
  if (args.outcome) {
    const o = String(args.outcome).toLowerCase();
    items = items.filter((p) => (p.last_outcome || '').toLowerCase() === o);
  }
  if (args.needs_rework === true || args.needs_rework === 'true') {
    items = items.filter((p) => p.needs_rework);
  }
  if (args.storage) {
    items = items.filter((p) => (p.storage || 'hot') === args.storage);
  }
  if (args.pack || args.pack_id) {
    const pack = sanitizePackId(args.pack || args.pack_id);
    items = items.filter((p) => sanitizePackId(p.pack_id || 'inbox') === pack);
  }
  if (args.subject_class || args.class) {
    const sc = String(args.subject_class || args.class).toLowerCase();
    items = items.filter((p) => inferSubjectClass(p) === sc);
  }
  // Prefer rework / pending scars first
  items = [...items].sort((a, b) => {
    const score = (p) =>
      (p.needs_rework ? 100 : 0) +
      (!p.last_outcome && (p.copy_count_without_scar || 0) > 0 ? 50 : 0) +
      ((p.storage || 'hot') === 'hot' ? 10 : 0);
    return score(b) - score(a);
  });
  return {
    count: items.length,
    rework_queue: reworkIds(lib),
    next_experiment: lib.next_experiment || null,
    packs: listPacks().packs,
    prompts: items.map((p) => ({
      id: p.id,
      title: p.title,
      tier: p.tier,
      tags: p.tags,
      pack_id: p.pack_id || 'inbox',
      subject_class: inferSubjectClass(p),
      last_outcome: p.last_outcome || null,
      needs_rework: !!p.needs_rework,
      storage: p.storage || 'hot',
      prompt_preview: (p.prompt || '').slice(0, 120),
      image: p.image || null,
      has_image: !!(p.image || (p.images && p.images.length)),
      updated_at: p.updated_at,
    })),
  };
}

function getLibraryPrompt(id) {
  const lib = loadLibrary();
  const p = (lib.prompts || []).find((x) => x.id === id);
  if (!p) throw new Error(`prompt not found: ${id}`);
  return p;
}

function savePrompt(args) {
  const prompt = String(args.prompt || '').trim();
  if (!prompt) throw new Error('prompt is required');
  const lib = loadLibrary();
  const now = nowIso();
  const cousins = findCousins(lib, prompt, args.id || null);
  let entry;
  if (args.id) {
    const idx = (lib.prompts || []).findIndex((x) => x.id === args.id);
    if (idx < 0) throw new Error(`prompt not found: ${args.id}`);
    entry = {
      ...lib.prompts[idx],
      title: args.title ?? lib.prompts[idx].title,
      tier: args.tier ?? lib.prompts[idx].tier,
      tags: args.tags ?? lib.prompts[idx].tags,
      prompt,
      notes: args.notes ?? lib.prompts[idx].notes,
      updated_at: now,
    };
    if (args.storage) {
      entry.storage = args.storage;
      if (args.storage === 'compost') entry.needs_rework = false;
    }
    if (args.skeleton) entry.skeleton = args.skeleton;
    if (args.fragment_ids) entry.fragment_ids = args.fragment_ids;
    if (args.pack_id || args.pack) {
      entry.pack_id = sanitizePackId(args.pack_id || args.pack);
    }
    if (args.subject_class) entry.subject_class = String(args.subject_class).toLowerCase();
    else entry.subject_class = inferSubjectClass(entry);
    if (args.image !== undefined) {
      entry.image = args.image ? String(args.image) : null;
    }
    if (args.images) entry.images = Array.isArray(args.images) ? args.images : [];
    lib.prompts[idx] = entry;
  } else {
    entry = {
      id: randomUUID(),
      title: args.title || prompt.slice(0, 48),
      tier: args.tier || critiquePrompt(prompt).tier,
      tags: Array.isArray(args.tags) ? args.tags : [],
      prompt,
      notes: args.notes || '',
      created_at: now,
      updated_at: now,
      last_outcome: null,
      last_note: '',
      last_run_at: null,
      last_disposition_at: null,
      copy_count_without_scar: 0,
      needs_rework: false,
      storage: args.storage || 'hot',
      skeleton: args.skeleton || null,
      fragment_ids: args.fragment_ids || [],
      pack_id: sanitizePackId(args.pack_id || args.pack || 'inbox'),
      subject_class: args.subject_class
        ? String(args.subject_class).toLowerCase()
        : null,
      image: args.image ? String(args.image) : null,
      images: Array.isArray(args.images) ? args.images : [],
    };
    entry.subject_class = inferSubjectClass(entry);
    lib.prompts = lib.prompts || [];
    lib.prompts.unshift(entry);
  }

  // Weight flora by tier on save
  if (entry.fragment_ids?.length) {
    const flora = loadFlora();
    const delta = { SS: 2, S: 1, A: 0, B: -1, C: -2 }[String(entry.tier || '').toUpperCase()] ?? 0;
    if (delta) {
      bumpFlora(flora, entry.fragment_ids, delta);
      saveFlora(flora);
    }
  }

  saveLibrary(lib);
  return {
    saved: true,
    entry,
    cousins,
    cousin_warning:
      cousins.length > 0
        ? 'Near-duplicates detected. Merge or diverge intentionally before flooding the cabinet.'
        : null,
  };
}

function recordOutcome(args) {
  const id = args.id;
  if (!id) throw new Error('id is required');
  const outcome = String(args.outcome || '').toLowerCase();
  if (!['won', 'failed', 'ambiguous'].includes(outcome)) {
    throw new Error('outcome must be won | failed | ambiguous');
  }
  const lib = loadLibrary();
  const idx = (lib.prompts || []).findIndex((x) => x.id === id);
  if (idx < 0) throw new Error(`prompt not found: ${id}`);
  const now = nowIso();
  const note = args.note != null ? String(args.note) : lib.prompts[idx].last_note || '';
  lib.prompts[idx].last_outcome = outcome;
  lib.prompts[idx].last_note = note;
  lib.prompts[idx].last_disposition_at = now;
  lib.prompts[idx].last_run_at = now;
  lib.prompts[idx].copy_count_without_scar = 0;
  lib.prompts[idx].updated_at = now;

  if (outcome === 'failed' || outcome === 'ambiguous') {
    lib.prompts[idx].needs_rework = true;
    lib.next_experiment = {
      prompt_id: id,
      action: 'rework',
      note: note || `Rework after ${outcome}`,
      status: 'open',
      updated_at: now,
    };
  } else {
    lib.prompts[idx].needs_rework = false;
    const flora = loadFlora();
    bumpFlora(flora, lib.prompts[idx].fragment_ids || [], 2);
    saveFlora(flora);
  }

  saveLibrary(lib);
  return { recorded: true, entry: lib.prompts[idx], rework_queue: reworkIds(lib) };
}

function markCopied(args) {
  const id = args.id;
  if (!id) throw new Error('id is required');
  const lib = loadLibrary();
  const idx = (lib.prompts || []).findIndex((x) => x.id === id);
  if (idx < 0) throw new Error(`prompt not found: ${id}`);
  const now = nowIso();
  lib.prompts[idx].last_run_at = now;
  if (!lib.prompts[idx].last_outcome) {
    lib.prompts[idx].copy_count_without_scar = (lib.prompts[idx].copy_count_without_scar || 0) + 1;
  }
  lib.prompts[idx].updated_at = now;
  lib.next_experiment = {
    prompt_id: id,
    action: 'rework',
    note: `Log outcome for: ${lib.prompts[idx].title || id}`,
    status: 'open',
    updated_at: now,
  };
  saveLibrary(lib);
  return {
    marked: true,
    cold_nag: !lib.prompts[idx].last_outcome && lib.prompts[idx].copy_count_without_scar >= 2,
    entry: lib.prompts[idx],
    next_experiment: lib.next_experiment,
  };
}

function getNextExperiment() {
  const lib = loadLibrary();
  return {
    next_experiment: lib.next_experiment || null,
    rework_queue: reworkIds(lib),
    experiment_history: (lib.experiment_history || []).slice(0, 10),
  };
}

/** Pin a mission, or finish it with `finish: "done"|"dismissed"`. */
function setNextExperiment(args) {
  const lib = loadLibrary();
  const now = nowIso();
  if (args.finish) {
    const status = args.finish === 'done' ? 'done' : 'dismissed';
    if (!lib.next_experiment) {
      return { completed: false, reason: 'no open experiment' };
    }
    lib.experiment_history = lib.experiment_history || [];
    lib.experiment_history.unshift({
      prompt_id: lib.next_experiment.prompt_id,
      action: lib.next_experiment.action,
      note: args.note != null ? String(args.note) : lib.next_experiment.note,
      status,
      closed_at: now,
    });
    lib.experiment_history = lib.experiment_history.slice(0, 40);
    lib.next_experiment = null;
    saveLibrary(lib);
    return {
      completed: true,
      next_experiment: null,
      experiment_history: lib.experiment_history.slice(0, 5),
    };
  }
  if (!args.prompt_id) throw new Error('prompt_id is required (or finish: "done"|"dismissed")');
  lib.next_experiment = {
    prompt_id: String(args.prompt_id),
    action: String(args.action || 'custom'),
    note: String(args.note || ''),
    status: 'open',
    updated_at: now,
  };
  saveLibrary(lib);
  return { set: true, next_experiment: lib.next_experiment };
}

function listFlora(args = {}) {
  const flora = loadFlora();
  let items = flora.fragments || [];
  if (args.slot) items = items.filter((f) => f.slot === args.slot);
  if (args.pool) items = items.filter((f) => f.pool === args.pool);
  if (args.style) {
    items = items.filter(
      (f) => !f.style_affinity?.length || f.style_affinity.includes(args.style),
    );
  }
  items = [...items].sort((a, b) => (b.weight || 0) - (a.weight || 0));
  return { count: items.length, fragments: items };
}

function roulette(args = {}) {
  const lib = loadLibrary();
  const styles = loadStyles();
  const prompts = (lib.prompts || []).filter((p) => (p.storage || 'hot') !== 'compost');
  const packs = styles.styles || [];
  if (!prompts.length || !packs.length) throw new Error('need prompts and styles');
  const p = prompts[Math.floor(Math.random() * prompts.length)];
  const s = packs[Math.floor(Math.random() * packs.length)];
  const core = splitClauses(p.prompt).slice(0, Number(args.core_clauses) || 3);
  const phrases = (s.phrases || []).slice(0, 2);
  const mash = [...core, ...phrases].filter(Boolean).join(', ');
  return {
    source_prompt_id: p.id,
    source_title: p.title,
    style_id: s.id,
    style_name: s.name,
    prompt: mash,
    prose_variant: toProse([...core, ...phrases], s.id),
  };
}

function deletePrompt(id) {
  const lib = loadLibrary();
  const before = (lib.prompts || []).length;
  lib.prompts = (lib.prompts || []).filter((x) => x.id !== id);
  if (lib.prompts.length === before) throw new Error(`prompt not found: ${id}`);
  saveLibrary(lib);
  return { deleted: true, id };
}

function getHandbook() {
  return {
    name: 'Mor Image Prompt Atelier',
    version: VERSION,
    purpose:
      'Living local desk for image prompts: draft, critique, ship, scar outcomes, rework, and compound craft DNA (flora).',
    practice_loop: [
      'Draft subject-first (slots).',
      'build_prompt / improve_prompt: flora fills empty lighting/medium (max 2).',
      'Copy / generate externally (Imagine etc.).',
      'record_outcome won|failed|ambiguous + note.',
      'Failed/ambiguous goes to rework; improve with last_note context.',
      'set_next_experiment before ending a session.',
      'Cull: save_prompt with storage cold|compost; keep hot shelf small.',
    ],
    workflow: [
      '1. get_next_experiment / list_library (rework first).',
      '2. list_styles / list_flora for craft DNA.',
      '3. build_prompt with slots OR improve_prompt on a rough draft.',
      '4. critique_prompt to see remaining gaps.',
      '5. vary_prompt or roulette for alternatives.',
      '6. save_prompt (cousin warnings if near-duplicates; storage for shelf).',
      '7. mark_copied when shipping; record_outcome after the image returns.',
      '8. set_next_experiment; finish: "done" when finished.',
    ],
    craft_order: 'subject > action/pose > setting > style > medium > composition > lighting/mood',
    mor_flavor: [
      'PC-98 / photocopied manga texture, stippled shadows, black background',
      'Dark academia, scholarly noir, foggy streets',
      'Mucha / art nouveau elevation of odd subjects',
      'Pen and ink, delicate linework',
    ],
    imagine_rules: [
      '2-5 natural sentences preferred over tag soup for Imagine',
      'Describe what to include, never negative prompts',
      'One coherent scene per prompt',
      'Match aspect_ratio to use case (16:9 banner, 9:16 story, 1:1 icon)',
    ],
    data_paths: {
      packs: PACKS_DIR,
      desk: DESK_PATH,
      library_export: LIBRARY_PATH,
      styles: STYLES_PATH,
      flora: FLORA_PATH,
      data_dir: DATA_DIR,
    },
    layout:
      'packs/<id>/prompts/*.json (content) + desk.json (missions) + library.json (flat export). Classification: pack_id, tier, storage, subject_class, tags.',
    tools: TOOLS.map((t) => t.name),
  };
}

// ── Tools schema ────────────────────────────────────────────────────────────

const TOOLS = [
  {
    name: 'get_handbook',
    description:
      'Read the Mor image-prompt craft handbook: workflow, style flavor, Imagine rules, data paths. Call first when starting prompt work.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'list_styles',
    description:
      'List style packs (PC-98, dark academia, Mucha, etc.) with phrase banks, plus media/lighting/composition lexicons.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string', description: 'Optional style id to fetch one pack' },
      },
    },
  },
  {
    name: 'critique_prompt',
    description:
      'Score an image prompt (0-100, tier SS to C), list present/missing craft slots, detect styles, and recommend fixes. Use before improving.',
    inputSchema: {
      type: 'object',
      properties: {
        prompt: { type: 'string', description: 'The image prompt to critique' },
      },
      required: ['prompt'],
    },
  },
  {
    name: 'improve_prompt',
    description:
      'Structurally improve an image prompt: fill missing slots, strip weak negatives, optionally apply a style pack. Returns improved clause list + prose_variant.',
    inputSchema: {
      type: 'object',
      properties: {
        prompt: { type: 'string', description: 'Draft prompt to improve' },
        style: {
          type: 'string',
          description: 'Style pack id from list_styles (e.g. pc98, mucha, dark-academia)',
        },
        intensity: {
          type: 'string',
          description: 'lean | balanced | rich (default balanced)',
        },
        aspect_ratio: {
          type: 'string',
          description: 'Optional 1:1, 16:9, 9:16, 3:2, 2:3. Adds usage hint only',
        },
        extra: { type: 'string', description: 'Extra clause to force-include' },
      },
      required: ['prompt'],
    },
  },
  {
    name: 'build_prompt',
    description:
      'Assemble a prompt from structured slots. Empty lighting/medium pull up to 2 weighted flora fragments.',
    inputSchema: {
      type: 'object',
      properties: {
        subject: { type: 'string' },
        action: { type: 'string' },
        setting: { type: 'string' },
        style: { type: 'string', description: 'Style pack id' },
        style_phrase: { type: 'string', description: 'Raw style phrase if not using a pack' },
        medium: { type: 'string' },
        composition: { type: 'string' },
        lighting: { type: 'string' },
        extra: { type: 'string' },
      },
      required: ['subject'],
    },
  },
  {
    name: 'vary_prompt',
    description:
      'Generate style/lighting/medium variations of a core prompt for A/B exploration.',
    inputSchema: {
      type: 'object',
      properties: {
        prompt: { type: 'string' },
        count: { type: 'number', description: '1-6 variations (default 3)' },
      },
      required: ['prompt'],
    },
  },
  {
    name: 'list_library',
    description:
      'List saved prompts (rework/pending scars first). Filters: q, tag, tier, outcome, needs_rework, storage, pack, subject_class.',
    inputSchema: {
      type: 'object',
      properties: {
        q: { type: 'string', description: 'Search title/prompt/tags/notes/pack/class' },
        tag: { type: 'string', description: 'Exact tag filter' },
        tier: { type: 'string', description: 'SS | S | A | B | C' },
        outcome: { type: 'string', description: 'won | failed | ambiguous' },
        needs_rework: { type: 'boolean' },
        storage: { type: 'string', description: 'hot | cold | compost' },
        pack: { type: 'string', description: 'Pack id (e.g. murdoch-core, characters)' },
        pack_id: { type: 'string', description: 'Alias for pack' },
        subject_class: {
          type: 'string',
          description: 'character | animal | scene | poster | other',
        },
      },
    },
  },
  {
    name: 'list_packs',
    description:
      'List prompt packs (shelves) with counts. Layout: data/packs/<id>/prompts/*.json.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'get_prompt',
    description: 'Fetch a full saved prompt by id (includes scars, skeleton, fragment_ids).',
    inputSchema: {
      type: 'object',
      properties: { id: { type: 'string' } },
      required: ['id'],
    },
  },
  {
    name: 'save_prompt',
    description:
      'Save or update a prompt. Auto-tiers if omitted. Returns cousin near-duplicate warnings. Bumps linked flora weights by tier.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string', description: 'Existing id to update' },
        title: { type: 'string' },
        prompt: { type: 'string' },
        tier: { type: 'string' },
        tags: {
          type: 'array',
          items: { type: 'string' },
        },
        notes: { type: 'string' },
        storage: {
          type: 'string',
          description: 'hot | cold | compost (shelf; compost clears needs_rework)',
        },
        pack_id: {
          type: 'string',
          description: 'Deck id (default inbox). e.g. murdoch-core, characters, poster-icons',
        },
        pack: { type: 'string', description: 'Alias for pack_id' },
        subject_class: {
          type: 'string',
          description: 'character | animal | scene | poster | other (auto-inferred if omitted)',
        },
        image: {
          type: 'string',
          description: 'Filename under packs/<deck>/media/ for the card face (e.g. gen.png)',
        },
        images: {
          type: 'array',
          items: { type: 'string' },
          description: 'Extra media filenames under the same media/ folder',
        },
        skeleton: {
          type: 'object',
          description: '{ subject, action, setting } for JIT re-assembly',
        },
        fragment_ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'Linked flora fragment ids',
        },
      },
      required: ['prompt'],
    },
  },
  {
    name: 'delete_prompt',
    description: 'Delete a saved prompt by id.',
    inputSchema: {
      type: 'object',
      properties: { id: { type: 'string' } },
      required: ['id'],
    },
  },
  {
    name: 'record_outcome',
    description:
      'Scar a prompt after external generation (won|failed|ambiguous). Failed/ambiguous set needs_rework + next_experiment.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string' },
        outcome: { type: 'string', description: 'won | failed | ambiguous' },
        note: { type: 'string', description: 'Why it worked or failed' },
      },
      required: ['id', 'outcome'],
    },
  },
  {
    name: 'mark_copied',
    description:
      'Mark that a prompt was shipped/copied for generation. Soft cold-nag after 2 copies without a scar. Sets next_experiment.',
    inputSchema: {
      type: 'object',
      properties: { id: { type: 'string' } },
      required: ['id'],
    },
  },
  {
    name: 'get_next_experiment',
    description: 'Read the open What\'s next mission, rework ids (from needs_rework), and recent history.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'set_next_experiment',
    description:
      'Pin a What\'s next mission (prompt_id + note), or finish it with finish: "done"|"dismissed".',
    inputSchema: {
      type: 'object',
      properties: {
        prompt_id: { type: 'string' },
        action: { type: 'string', description: 'Free-form label (e.g. rework, custom)' },
        note: { type: 'string' },
        finish: {
          type: 'string',
          description: 'done | dismissed: archive the open mission',
        },
      },
    },
  },
  {
    name: 'list_flora',
    description: 'List micro-prompt flora fragments (craft DNA) with weights, slots, pools.',
    inputSchema: {
      type: 'object',
      properties: {
        slot: { type: 'string', description: 'lighting | medium | composition | setting' },
        pool: { type: 'string', description: 'murdoch-core | experimental' },
        style: { type: 'string', description: 'Filter by style_affinity' },
      },
    },
  },
  {
    name: 'roulette',
    description: 'Random library prompt core x random style pack. Serendipity mashup.',
    inputSchema: {
      type: 'object',
      properties: {
        core_clauses: { type: 'number', description: 'How many leading clauses to keep (default 3)' },
      },
    },
  },
];

// ── Tool dispatch ───────────────────────────────────────────────────────────

function callTool(name, args = {}) {
  switch (name) {
    case 'get_handbook':
      return getHandbook();
    case 'list_styles': {
      const data = loadStyles();
      if (args.id) {
        const s = (data.styles || []).find((x) => x.id === args.id);
        if (!s) throw new Error(`unknown style: ${args.id}`);
        return s;
      }
      return {
        styles: (data.styles || []).map((s) => ({
          id: s.id,
          name: s.name,
          tags: s.tags,
          phrase_count: (s.phrases || []).length,
          notes: s.notes,
        })),
        media: data.media,
        lighting: data.lighting,
        composition: data.composition,
        aspect_hints: data.aspect_hints,
      };
    }
    case 'critique_prompt':
      return critiquePrompt(args.prompt);
    case 'improve_prompt':
      return improvePrompt(args);
    case 'build_prompt':
      return buildPrompt(args);
    case 'vary_prompt':
      return varyPrompt(args);
    case 'list_library':
      return listLibrary(args);
    case 'list_packs':
      return listPacks();
    case 'get_prompt':
      return getLibraryPrompt(args.id);
    case 'save_prompt':
      return savePrompt(args);
    case 'delete_prompt':
      return deletePrompt(args.id);
    case 'record_outcome':
      return recordOutcome(args);
    case 'mark_copied':
      return markCopied(args);
    case 'get_next_experiment':
      return getNextExperiment();
    case 'set_next_experiment':
      return setNextExperiment(args);
    case 'list_flora':
      return listFlora(args);
    case 'roulette':
      return roulette(args);
    default:
      throw new Error(`unknown tool ${name}`);
  }
}

// ── stdio JSON-RPC (MCP) ────────────────────────────────────────────────────

const reply = (id, result) =>
  process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result }) + '\n');

const replyError = (id, code, message) =>
  process.stdout.write(
    JSON.stringify({ jsonrpc: '2.0', id, error: { code, message } }) + '\n',
  );

async function handle(msg) {
  const { id, method, params } = msg;
  if (method === 'initialize') {
    return reply(id, {
      protocolVersion: params?.protocolVersion ?? '2024-11-05',
      capabilities: { tools: {} },
      serverInfo: { name: 'mor-image-prompts', version: VERSION },
    });
  }
  if (method === 'notifications/initialized' || method === 'initialized') return;
  if (method === 'ping') return reply(id, {});
  if (method === 'tools/list') return reply(id, { tools: TOOLS });
  if (method === 'tools/call') {
    try {
      const result = callTool(params.name, params.arguments ?? {});
      return reply(id, {
        content: [{ type: 'text', text: JSON.stringify(result, null, 2) }],
      });
    } catch (e) {
      return reply(id, {
        content: [{ type: 'text', text: `Error: ${e.message || e}` }],
        isError: true,
      });
    }
  }
  if (id !== undefined) replyError(id, -32601, `Method not found: ${method}`);
}

// Self-check mode
if (process.argv.includes('--check')) {
  ensureData();
  const h = getHandbook();
  const c = critiquePrompt(
    'a grey pitbull in the style of Alphonse Mucha, art nouveau poster',
  );
  const i = improvePrompt({ prompt: 'sad wizard', style: 'pc98' });
  const b = buildPrompt({ subject: 'three wordsmiths at a typewriter', style: 'pc98' });
  const flora = listFlora({ pool: 'murdoch-core' });
  const roul = roulette({});
  const packs = listPacks();
  const lib = listLibrary({});
  console.log(
    JSON.stringify(
      {
        ok: true,
        version: VERSION,
        handbook_tools: h.tools,
        critique_tier: c.tier,
        improved: i.improved,
        build_flora: b.flora_used,
        flora_count: flora.count,
        roulette_style: roul.style_id,
        pack_count: packs.count,
        prompt_count: lib.count,
        layout: h.layout,
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

ensureData();

let buf = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  buf += chunk;
  let idx;
  while ((idx = buf.indexOf('\n')) >= 0) {
    const line = buf.slice(0, idx).trim();
    buf = buf.slice(idx + 1);
    if (!line) continue;
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      continue;
    }
    // Content-Length framing not used; newline-delimited JSON like sentence-miner
    Promise.resolve(handle(msg)).catch((e) => {
      if (msg?.id !== undefined) replyError(msg.id, -32000, String(e.message || e));
    });
  }
});

process.stdin.on('end', () => process.exit(0));
