#!/usr/bin/env node
// Mor Image Prompt Atelier MCP — stdio, zero deps.
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
  readFileSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { homedir } from 'node:os';

const VERSION = '0.1.0';
const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');

const DATA_DIR =
  process.env.MOR_PROMPTS_DATA ||
  join(ROOT, 'data');
const LIBRARY_PATH = join(DATA_DIR, 'library.json');
const STYLES_PATH = join(DATA_DIR, 'styles.json');

// ── IO ──────────────────────────────────────────────────────────────────────

function ensureData() {
  mkdirSync(DATA_DIR, { recursive: true });
  if (!existsSync(LIBRARY_PATH)) {
    writeFileSync(
      LIBRARY_PATH,
      JSON.stringify({ version: 1, prompts: [] }, null, 2) + '\n',
    );
  }
  if (!existsSync(STYLES_PATH)) {
    writeFileSync(
      STYLES_PATH,
      JSON.stringify({ version: 1, styles: [], media: [], lighting: [], composition: [], aspect_hints: {} }, null, 2) + '\n',
    );
  }
}

function loadLibrary() {
  ensureData();
  return JSON.parse(readFileSync(LIBRARY_PATH, 'utf8'));
}

function saveLibrary(lib) {
  ensureData();
  writeFileSync(LIBRARY_PATH, JSON.stringify(lib, null, 2) + '\n');
}

function loadStyles() {
  ensureData();
  return JSON.parse(readFileSync(STYLES_PATH, 'utf8'));
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
  if (det.word_count < 12) recommendations.push('Expand slightly — aim for 2–5 natural sentences or a tight clause list.');
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
    craft_order: 'subject → action/pose → setting → style → medium → composition → lighting/mood',
  };
}

function pick(arr, i = 0) {
  if (!arr?.length) return '';
  return arr[i % arr.length];
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
    additions.push(mediumish || pick(stylesData.media, 0));
  }
  if (!critique.present_slots.includes('lighting')) {
    if (goalStyle === 'pc98') additions.push('stippled shadows');
    else if (goalStyle === 'noir' || goalStyle === 'dark-academia') additions.push('candlelight and long shadows');
    else additions.push(pick(stylesData.lighting, 2));
  }
  if (!critique.present_slots.includes('composition') && intensity !== 'lean') {
    if (goalStyle === 'mucha') additions.push('art nouveau poster composition, ornate frame');
    else additions.push(pick(stylesData.composition, 0));
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

  // Reorder: keep original subject first, then rest
  let improved = merged.join(', ');
  if (intensity === 'rich' && !/[.!?]$/.test(improved)) {
    // leave as clause list — natural for image models
  }

  const altLean = merged.slice(0, Math.min(5, merged.length)).join(', ');
  const altRich = (() => {
    const more = [...merged];
    if (goalStyle) {
      for (const p of stylePhrases(goalStyle, stylesData).slice(0, 3)) {
        if (!more.some((m) => m.toLowerCase().includes(p.toLowerCase()))) more.push(p);
      }
    }
    return more.join(', ');
  })();

  // Prose form alternative (Imagine prefers 2–5 sentences)
  const prose = toProse(merged, goalStyle);

  return {
    original: prompt,
    improved,
    prose_variant: prose,
    alternatives: [
      { id: 'lean', prompt: altLean },
      { id: 'rich', prompt: altRich },
      { id: 'prose', prompt: prose },
    ],
    critique,
    applied_style: goalStyle,
    aspect_ratio_hint: aspect
      ? { aspect_ratio: aspect, use: stylesData.aspect_hints?.[aspect] || null }
      : null,
    notes_for_agent: [
      'Prefer the improved clause list for tag-style models; use prose_variant for Imagine/Grok image tools.',
      'If generating with Imagine: front-load subject, one scene, no negative prompts.',
      'Save keepers with save_prompt.',
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
  const parts = [];
  if (args.subject) parts.push(String(args.subject).trim());
  if (args.action) parts.push(String(args.action).trim());
  if (args.setting) parts.push(String(args.setting).trim());

  const styleId = args.style || null;
  if (styleId) {
    const phrases = stylePhrases(styleId, stylesData);
    if (args.include_style_phrases !== false && phrases.length) {
      parts.push(phrases[0]);
      if (args.rich) parts.push(...phrases.slice(1, 3));
    } else {
      parts.push(styleId);
    }
  } else if (args.style_phrase) {
    parts.push(String(args.style_phrase).trim());
  }

  if (args.medium) parts.push(String(args.medium).trim());
  else if (args.rich) parts.push(pick(stylesData.media, 0));

  if (args.composition) parts.push(String(args.composition).trim());
  if (args.lighting) parts.push(String(args.lighting).trim());
  else if (args.rich) parts.push(pick(stylesData.lighting, 2));

  if (args.extra) parts.push(String(args.extra).trim());

  const clause = parts.filter(Boolean).join(', ');
  const critique = critiquePrompt(clause);
  return {
    prompt: clause,
    prose_variant: toProse(parts.filter(Boolean), styleId),
    critique,
    slots_used: {
      subject: args.subject || null,
      action: args.action || null,
      setting: args.setting || null,
      style: styleId,
      medium: args.medium || null,
      composition: args.composition || null,
      lighting: args.lighting || null,
    },
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
  // strip trailing style-ish tails loosely: keep first 2–4 clauses as core
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
        (p.tags || []).some((x) => x.toLowerCase().includes(q)),
    );
  }
  if (args.tier) {
    items = items.filter((p) => (p.tier || '').toUpperCase() === String(args.tier).toUpperCase());
  }
  return {
    count: items.length,
    prompts: items.map((p) => ({
      id: p.id,
      title: p.title,
      tier: p.tier,
      tags: p.tags,
      prompt_preview: (p.prompt || '').slice(0, 120),
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
  const now = new Date().toISOString();
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
    };
    lib.prompts = lib.prompts || [];
    lib.prompts.unshift(entry);
  }
  saveLibrary(lib);
  return { saved: true, entry };
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
    purpose: 'Draft, critique, improve, and store image-generation prompts for Grok / Imagine / local atelier.',
    workflow: [
      '1. Optional: list_styles / list_library for inspiration.',
      '2. build_prompt with slots OR improve_prompt on a rough draft.',
      '3. critique_prompt to see remaining gaps.',
      '4. vary_prompt for style/lighting alternatives.',
      '5. save_prompt when a keeper is ready.',
      '6. For image generation: use prose_variant with Imagine; front-load subject; one scene; no negatives.',
    ],
    craft_order: 'subject → action/pose → setting → style → medium → composition → lighting/mood',
    mor_flavor: [
      'PC-98 / photocopied manga texture, stippled shadows, black background',
      'Dark academia, scholarly noir, foggy streets',
      'Mucha / art nouveau elevation of odd subjects',
      'Pen and ink, delicate linework',
    ],
    imagine_rules: [
      '2–5 natural sentences preferred over tag soup for Imagine',
      'Describe what to include, never negative prompts',
      'One coherent scene per prompt',
      'Match aspect_ratio to use case (16:9 banner, 9:16 story, 1:1 icon)',
    ],
    data_paths: {
      library: LIBRARY_PATH,
      styles: STYLES_PATH,
      data_dir: DATA_DIR,
    },
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
      'List style packs (PC-98, dark academia, Mucha, …) with phrase banks, plus media/lighting/composition lexicons.',
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
      'Score an image prompt (0–100, tier SS–C), list present/missing craft slots, detect styles, and recommend fixes. Use before improving.',
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
      'Structurally improve an image prompt: fill missing slots, strip weak negatives, optionally apply a style pack. Returns improved clause list, prose variant, and alternatives (lean/rich/prose).',
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
          description: 'Optional 1:1, 16:9, 9:16, 3:2, 2:3 — adds usage hint only',
        },
        extra: { type: 'string', description: 'Extra clause to force-include' },
      },
      required: ['prompt'],
    },
  },
  {
    name: 'build_prompt',
    description:
      'Assemble a prompt from structured slots (subject, action, setting, style, medium, composition, lighting).',
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
        rich: { type: 'boolean', description: 'Pull extra phrases from style pack' },
        include_style_phrases: {
          type: 'boolean',
          description: 'Default true — inject pack phrases when style id given',
        },
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
        count: { type: 'number', description: '1–6 variations (default 3)' },
      },
      required: ['prompt'],
    },
  },
  {
    name: 'list_library',
    description: 'List saved prompts in the local atelier library (id, title, tier, tags, preview).',
    inputSchema: {
      type: 'object',
      properties: {
        q: { type: 'string', description: 'Search title/prompt/tags/notes' },
        tag: { type: 'string', description: 'Exact tag filter' },
        tier: { type: 'string', description: 'SS | S | A | B | C' },
      },
    },
  },
  {
    name: 'get_prompt',
    description: 'Fetch a full saved prompt by id.',
    inputSchema: {
      type: 'object',
      properties: { id: { type: 'string' } },
      required: ['id'],
    },
  },
  {
    name: 'save_prompt',
    description:
      'Save a new prompt or update an existing one (pass id to update). Auto-tiers from critique if tier omitted.',
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
    case 'get_prompt':
      return getLibraryPrompt(args.id);
    case 'save_prompt':
      return savePrompt(args);
    case 'delete_prompt':
      return deletePrompt(args.id);
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
  console.log(JSON.stringify({ ok: true, version: VERSION, handbook_tools: h.tools, critique_tier: c.tier, improved: i.improved }, null, 2));
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
