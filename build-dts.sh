#!/usr/bin/bash
set -ue
npx -p typescript tsc lib.js env.d.ts --lib esnext --declaration --allowJs --emitDeclarationOnly --outDir target
cat env.d.ts > target/deft.d.ts
echo "" >> target/deft.d.ts
cat target/lib.d.ts >> target/deft.d.ts
sed -i 's/export /declare /g' target/deft.d.ts
sed -i 's/declare {};//g' target/deft.d.ts
sed -i 's/#private;//g' target/deft.d.ts
rm target/lib.d.ts
