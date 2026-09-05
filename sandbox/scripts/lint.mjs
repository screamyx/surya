import { readFileSync, readdirSync, lstatSync } from 'node:fs';
import { resolve, relative, extname } from 'node:path';
import { pathToFileURL } from 'node:url';
import ts from 'typescript';
const root = resolve(import.meta.dirname, '..');
const whitelist = JSON.parse(readFileSync(resolve(root, 'whitelist.json'), 'utf8'));
export function allowedClass(value) {
  const parts = value.split(':');
  if (parts.length > 2 || (parts.length === 2 && !(`${parts[0]}:` in whitelist.variants))) return false;
  const cls = parts.at(-1);
  if (Object.hasOwn(whitelist.exact, cls)) return true;
  return whitelist.families.some(f => {
    if (!cls.startsWith(f.prefix)) return false;
    const [suffix, alpha, extra] = cls.slice(f.prefix.length).split('/');
    if (!f.suffixes.includes(suffix) || extra !== undefined) return false;
    return alpha === undefined || (f.alpha?.includes(Number(alpha)) && String(Number(alpha)) === alpha
      && (!f.alphaSuffixes || f.alphaSuffixes.includes(suffix)));
  });
}
export function lintSource(source, file = 'src/Example.tsx') {
  const ast = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const errors = [];
  const fail = (node, message) => errors.push(`${file}:${ast.getLineAndCharacterOfPosition(node.getStart(ast)).line + 1}: ${message}`);
  function classes(node) {
    if (ts.isJsxExpression(node) || ts.isParenthesizedExpression(node)) {
      if (node.expression) classes(node.expression); else fail(node, 'Empty class expression');
    } else if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      for (const cls of node.text.trim().split(/\s+/).filter(Boolean)) {
        if (!allowedClass(cls)) fail(node, `Class not in whitelist: ${cls}`);
      }
    } else if (ts.isConditionalExpression(node)) {
      classes(node.whenTrue); classes(node.whenFalse);
    } else fail(node, 'Classes must be literals or ternaries of full literal lists');
  }
  function visit(node) {
    if (ts.isImportDeclaration(node)) {
      const spec = node.moduleSpecifier.text;
      const clause = node.importClause;
      if (spec.startsWith('.') && !resolve(root, file, '..', spec).startsWith(resolve(root, 'src') + '/')) fail(node, 'Imports must stay within src');
      if (spec === 'react') {
        if (clause?.name || !clause?.namedBindings || !ts.isNamedImports(clause.namedBindings)) fail(node, 'Only named allowed React imports');
        else for (const entry of clause.namedBindings.elements) {
          const name = (entry.propertyName ?? entry.name).text;
          const permitted = entry.isTypeOnly || clause.isTypeOnly ? whitelist.reactTypeImports : whitelist.reactImports;
          if (!permitted.includes(name)) fail(node, `React import not allowed: ${name}`);
        }
      } else if (spec === 'react-dom/client' && file === 'src/main.tsx') {
        if (clause?.name || !clause?.namedBindings || !ts.isNamedImports(clause.namedBindings)
          || clause.namedBindings.elements.some(e => (e.propertyName ?? e.name).text !== 'createRoot')) fail(node, 'Only createRoot in harness');
      } else if (!/^\.\.?\//.test(spec) || /\.(css|scss|sass|less)(\?|$)/i.test(spec) && !(file === 'src/main.tsx' && spec === './tailwind.css')) {
        fail(node, `Import not allowed: ${spec}`);
      }
    }
    if (ts.isExportDeclaration(node) && node.moduleSpecifier) fail(node, 'Re-exports cannot bypass the import boundary');
    if (ts.isJsxAttribute(node)) {
      const name = node.name.getText(ast);
      if (name === 'style' || name === 'dangerouslySetInnerHTML' || name === 'ref') fail(node, `${name} is not portable`);
      if (name === 'class' || name === 'className') {
        if (node.initializer) classes(node.initializer); else fail(node, 'Class needs a literal value');
      }
      if (['fill', 'stroke', 'color'].includes(name) && node.initializer
        && !(ts.isStringLiteral(node.initializer) && ['none', 'currentColor'].includes(node.initializer.text))) {
        fail(node, 'SVG paint must be none or currentColor');
      }
    }
    if (ts.isJsxSpreadAttribute(node)) fail(node, 'JSX spreads can hide classes and inline styles');
    if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
      if (['style', 'link', 'script', 'iframe'].includes(node.tagName.getText(ast))) fail(node, 'Embedded styling or external documents are banned');
    }
    if (ts.isCallExpression(node) && (node.expression.kind === ts.SyntaxKind.ImportKeyword
      || ['require', 'eval', 'Function', 'fetch', 'setTimeout', 'setInterval', 'requestAnimationFrame'].includes(node.expression.getText(ast)))) {
      fail(node, 'Dynamic imports, effects and fetching are banned');
    }
    if (ts.isNewExpression(node) && ['Function', 'WebSocket', 'EventSource', 'Worker'].includes(node.expression.getText(ast))) fail(node, 'Runtime effects are banned');
    if (ts.isPropertyAccessExpression(node)) {
      if (['style', 'classList', 'className', 'innerHTML', 'outerHTML', 'adoptedStyleSheets'].includes(node.name.text)) fail(node, 'DOM styling/mutation is banned');
      if (node.expression.getText(ast) === 'React') fail(node, 'React namespace calls bypass named imports');
    }
    if (ts.isIdentifier(node) && ['fetch', 'WebSocket', 'EventSource', 'Worker', 'eval', 'setTimeout', 'setInterval', 'requestAnimationFrame'].includes(node.text)) fail(node, 'Effects and fetching cannot be aliased');
    if (ts.isIdentifier(node) && ['window', 'document', 'globalThis', 'localStorage', 'sessionStorage', 'navigator'].includes(node.text)
      && file !== 'src/main.tsx') fail(node, 'Browser APIs belong only in the fixture bootstrap');
    ts.forEachChild(node, visit);
  }
  ast.parseDiagnostics.forEach(d => errors.push(`${file}: ${ts.flattenDiagnosticMessageText(d.messageText, ' ')}`));
  visit(ast);
  return errors;
}
export function lintTree(directory = root) {
  const errors = [];
  function walk(path) {
    for (const item of readdirSync(path)) {
      if (path === directory && ['node_modules', 'dist', '.git'].includes(item)) continue;
      const absolute = resolve(path, item), rel = relative(directory, absolute).replaceAll('\\', '/');
      const stat = lstatSync(absolute);
      if (stat.isSymbolicLink()) { errors.push(`${rel}: symlinks bypass source checks`); continue; }
      if (stat.isDirectory()) { walk(absolute); continue; }
      if (/\.(css|scss|sass|less)$/i.test(item) && rel !== 'src/tailwind.css') errors.push(`${rel}: extra stylesheet`);
      if (rel.startsWith('src/') && /\.[cm]?[jt]sx?$/.test(item)) errors.push(...lintSource(readFileSync(absolute, 'utf8'), rel));
      if (rel.startsWith('src/') && !['.tsx', '.ts', '.css'].includes(extname(item))) errors.push(`${rel}: unsupported source file`);
      if (/\.(tsx?|mjs|json|md|css|html)$/.test(item)) {
        const content = readFileSync(absolute, 'utf8');
        if (content.split('\n').length - 1 > 500) errors.push(`${rel}: over 500 lines`);
        if (/\p{Extended_Pictographic}|\uFE0F|\u20E3/u.test(content)) errors.push(`${rel}: emoji forbidden`);
        if (item.endsWith('.md') && content.includes('\u2014')) errors.push(`${rel}: em dash forbidden`);
      }
    }
  }
  walk(directory);
  return errors;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const errors = lintTree(process.argv[2] ? resolve(process.argv[2]) : root);
  if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
  else console.log('Portable source lint passed');
}
