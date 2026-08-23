const fs = require('fs');

function fixFile(file) {
  let content = fs.readFileSync(file, 'utf8');
  content = content.replace(/e\.message/g, '(e as Error).message');
  content = content.replace(/error\.message/g, '(error as Error).message');
  content = content.replace(/error\?\.message/g, '(error as Error)?.message');
  fs.writeFileSync(file, content);
}

fixFile('hydragrow-frontend/src/pages/Settings.tsx');
