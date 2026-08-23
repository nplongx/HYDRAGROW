const fs = require('fs');

function fixFile(file) {
  let content = fs.readFileSync(file, 'utf8');
  content = content.replace(/catch \(([^)]+): any\)/g, 'catch ($1: unknown)');
  if (file.includes('models.ts')) {
    content = content.replace(/\[key: string\]: any;/g, '[key: string]: unknown;');
  }
  fs.writeFileSync(file, content);
}

const files = [
  'hydragrow-frontend/src/pages/Settings.tsx',
  'hydragrow-frontend/src/pages/SystemLog.tsx',
  'hydragrow-frontend/src/types/models.ts'
];

files.forEach(fixFile);
