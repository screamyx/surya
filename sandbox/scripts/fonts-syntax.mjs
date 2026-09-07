// The two families and the syntax palette, read out of the Rust.
//
// Fonts: `ui/src/typography.rs` is the list of faces the app actually registers
// with gpui, so it decides what the sandbox ships and what @font-face rules the
// stylesheet carries. `theme.rs` names the families.
//
// Syntax: the active palette is not `SyntaxPalette::dark()`. `from_variant`
// overrides every field from the theme variant's twelve seed colors, so the
// resolution here follows seed array -> `builtins.rs::syntax()` map -> field.
import { block, fields, split, color } from './rust-colors.mjs';

const WEIGHTS = { Medium: 500, SemiBold: 600, Bold: 700 };

/** The registered faces of one family, from a `const NAME: [&[u8]; N]` list. */
export function faces(typography, constant) {
  const list = typography.match(new RegExp(`const ${constant}: \\[&\\[u8\\]; (\\d+)\\] = \\[([\\s\\S]*?)\\];`));
  if (!list) throw new Error(`typography.rs no longer registers ${constant}`);
  const files = [...list[2].matchAll(/assets\/fonts\/([\w-]+)\.ttf/g)].map(m => m[1]);
  if (files.length !== Number(list[1])) throw new Error(`${constant} lost a face`);
  return files.map(file => {
    const suffix = file.includes('-') ? file.slice(file.indexOf('-') + 1) : '';
    const italic = suffix.endsWith('Italic');
    const stem = italic ? suffix.slice(0, -'Italic'.length) : suffix;
    if (stem !== '' && !(stem in WEIGHTS)) throw new Error(`Unknown font face weight: ${file}`);
    return { file, weight: stem === '' ? 400 : WEIGHTS[stem], style: italic ? 'italic' : 'normal' };
  });
}

/** The family names, checked against the Rust the way the app sets them. */
export function families(theme) {
  const named = fields(block(block(theme, 'pub fn dark_with_accent'), 'Self'));
  const name = key => named[key]?.match(/"([^"]+)"/)?.[1];
  const sans = name('font_sans');
  const mono = name('font_mono');
  const fixed = name('font_sans_fixed');
  if (!sans || !mono || !fixed) throw new Error('Missing a font family name');
  // font_sans_fixed is a distinct Theme field that names the same family as
  // font_sans in both appearances (theme.rs asserts it at line 2571). It gets
  // no class of its own; if it ever diverges this throws instead of guessing.
  if (fixed !== sans) throw new Error(`font_sans_fixed is now ${fixed}, not ${sans}: it needs its own class`);
  return { sans, mono };
}

/** Every `SyntaxPalette` field, resolved for one theme variant's seed colors. */
export function syntaxPalette(theme, builtins, seedSyntax) {
  const fieldNames = [...block(theme, 'pub struct SyntaxPalette').matchAll(/pub (\w+): Hsla/g)].map(m => m[1]);
  // `fn syntax(colors: [&str; 12])` destructures the seed array, then names
  // each palette key after one of those slots.
  const body = block(builtins, 'fn syntax(colors: [&str; 12])');
  const slots = split(body.slice(body.indexOf('[') + 1, body.indexOf(']')), ',');
  const entries = Object.fromEntries([...body.matchAll(/\("(\w+)"\.into\(\), (\w+)\)/g)]
    .map(([, key, slot]) => [key, slots.indexOf(slot)]));
  if (Object.values(entries).includes(-1)) throw new Error('builtins.rs::syntax() names an unknown slot');
  // `from_variant` maps each field to one of those keys.
  const variantKey = Object.fromEntries([...block(theme, 'fn from_variant(variant: &ThemeVariant')
    .matchAll(/(\w+): color\("(\w+)", fallback\.\w+\)/g)].map(([, field, key]) => [field, key]));
  const seeds = split(seedSyntax.replace(/^\[|\]$/g, ''), ',').map(v => color(JSON.parse(v)));
  if (seeds.length !== slots.length) throw new Error('A theme variant seeds the wrong number of syntax colors');
  const out = {};
  for (const field of fieldNames) {
    const key = variantKey[field];
    // Every key the palette needs is in the map, so the oklch fallbacks in
    // SyntaxPalette::dark()/light() are never reached for a seeded variant.
    // If that stops being true the colour has to be computed, not guessed.
    if (key === undefined || entries[key] === undefined) {
      throw new Error(`syntax.${field} would fall back to an unevaluated Rust expression`);
    }
    out[field] = seeds[entries[key]];
  }
  return out;
}
