#!/usr/bin/bash
set -ue
npx -p typescript tsc lib.js env.d.ts --lib esnext --declaration --allowJs --emitDeclarationOnly --outDir js
cat env.d.ts > js/deft.d.ts
echo "" >> js/deft.d.ts
cat js/lib.d.ts >> js/deft.d.ts
sed -i 's/export /declare /g' js/deft.d.ts
sed -i 's/declare {};//g' js/deft.d.ts
sed -i 's/#private;//g' js/deft.d.ts
rm js/lib.d.ts
