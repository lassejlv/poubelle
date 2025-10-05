import { $, build } from 'bun'
import ora from 'ora'

const spinner = ora('Building...').start()

await build({
  target: 'bun',
  format: 'esm',
  outdir: 'dist',
  entrypoints: ['src/index.ts'],
  minify: {
    whitespace: true,
  },
  // external: Object.keys(pkg.dependencies),
})

await $`bun x tsc`

spinner.succeed('Build completed successfully')
