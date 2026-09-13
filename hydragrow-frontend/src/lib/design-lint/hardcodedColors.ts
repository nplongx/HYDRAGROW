const OFF_BRAND_HUES = [
  'blue', 'indigo', 'purple', 'pink', 'violet', 'fuchsia',
  'cyan', 'teal', 'orange', 'lime', 'rose',
];
const SHADES = ['50', '100', '200', '300', '400', '500', '600', '700', '800', '900', '950'];
const PROPS = ['bg', 'text', 'border', 'ring', 'fill', 'stroke'];

const PATTERN = new RegExp(
  `\\b(${PROPS.join('|')})-(${OFF_BRAND_HUES.join('|')})-(${SHADES.join('|')})\\b`,
  'g',
);

export interface ColorUsageViolation {
  file: string;
  line: number;
  match: string;
}

export function findHardcodedColorUsages(fileContent: string, filePath: string): ColorUsageViolation[] {
  const violations: ColorUsageViolation[] = [];
  const lines = fileContent.split('\n');
  lines.forEach((lineText, idx) => {
    const matches = lineText.match(PATTERN);
    if (matches) {
      matches.forEach((match) => violations.push({ file: filePath, line: idx + 1, match }));
    }
  });
  return violations;
}
