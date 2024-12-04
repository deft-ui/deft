#!/usr/bin/bash
set -ue
npx -p typescript tsc lib.js env.d.ts --lib esnext --declaration --allowJs --emitDeclarationOnly --outDir target
cat env.d.ts > target/lento.d.ts
echo "" >> target/lento.d.ts
cat target/lib.d.ts >> target/lento.d.ts
sed -i 's/export /declare /g' target/lento.d.ts
sed -i 's/declare {};//g' target/lento.d.ts
sed -i 's/#private;//g' target/lento.d.ts
rm target/lib.d.ts
