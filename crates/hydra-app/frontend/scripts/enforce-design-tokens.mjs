import fs from 'node:fs';
import path from 'node:path';

const projectRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const srcRoot = path.join(projectRoot, 'src');

const ALLOWED_PATH_PARTS = [
  `${path.sep}styles${path.sep}`,
  `${path.sep}generated${path.sep}`,
  `${path.sep}components${path.sep}design-system${path.sep}`,
];

const FILE_EXTENSIONS = new Set(['.ts', '.tsx']);
const hexPattern = /#[0-9a-fA-F]{3,8}\b/g;
const rgbPattern = /rgba?\(/g;

function shouldSkip(filePath) {
  return ALLOWED_PATH_PARTS.some((part) => filePath.includes(part));
}

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(fullPath, out);
      continue;
    }
    if (!FILE_EXTENSIONS.has(path.extname(entry.name))) {
      continue;
    }
    if (shouldSkip(fullPath)) {
      continue;
    }
    out.push(fullPath);
  }
  return out;
}

function collectViolations(filePath) {
  const text = fs.readFileSync(filePath, 'utf8');
  const violations = [];

  for (const match of text.matchAll(hexPattern)) {
    violations.push({
      filePath,
      token: match[0],
      rule: 'hex',
    });
  }

  for (const match of text.matchAll(rgbPattern)) {
    violations.push({
      filePath,
      token: match[0],
      rule: 'rgb',
    });
  }

  return violations;
}

const files = walk(srcRoot);
const violations = files.flatMap(collectViolations);

if (violations.length > 0) {
  console.error('Design token lint failed: found raw color literals in feature code.');
  for (const violation of violations) {
    const relative = path.relative(projectRoot, violation.filePath);
    console.error(`- ${relative}: disallowed ${violation.rule} token \`${violation.token}\``);
  }
  process.exit(1);
}

console.log(`Design token lint passed (${files.length} files checked).`);
