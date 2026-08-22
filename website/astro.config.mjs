import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://sv-stark.github.io',
  base: '/ebook-rs',
  integrations: [
    starlight({
      title: 'ebook-rs',
      description: 'Pure Rust multi-format eBook engine with AI RAG chunking, MCP server, SIMD search, and Python/WASM bindings.',
      logo: {
        src: './src/assets/logo.svg',
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/SV-stark/ebook-rs' },
      ],
      customCss: [
        './src/styles/custom.css',
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Introduction', slug: 'index' },
            { label: 'Quick Start', slug: 'getting-started/quickstart' },
            { label: 'Installation', slug: 'getting-started/installation' },
            { label: 'CLI Reference', slug: 'getting-started/cli' },
          ],
        },
        {
          label: 'Supported Formats',
          items: [
            { label: 'Overview & Matrix', slug: 'formats/overview' },
            { label: 'EPUB 2 & 3 (FXL)', slug: 'formats/epub' },
            { label: 'Amazon MOBI & AZW3', slug: 'formats/mobi-azw3' },
            { label: 'Amazon KFX (Clean-Room)', slug: 'formats/kfx' },
            { label: 'FB2 (FictionBook)', slug: 'formats/fb2' },
            { label: 'LIT (Microsoft Reader)', slug: 'formats/lit' },
            { label: 'PDF & Academic Reflow', slug: 'formats/pdf' },
            { label: 'Word DOCX & RTF', slug: 'formats/docx-rtf' },
            { label: 'Comics (CBZ / CBR)', slug: 'formats/comics' },
          ],
        },
        {
          label: 'Core Architecture',
          items: [
            { label: 'Zero-Alloc SIMD Search', slug: 'core/search' },
            { label: 'IDPF & Readium CFI Engine', slug: 'core/cfi' },
            { label: 'AI RAG & BM25 Scoring', slug: 'core/rag' },
            { label: 'DOM, XML & Tree-Sitter', slug: 'core/dom' },
            { label: 'Speech & SMIL Overlays', slug: 'core/tts-smil' },
            { label: 'Readium LCP DRM', slug: 'core/lcp' },
          ],
        },
        {
          label: 'Exporters & Optimizers',
          items: [
            { label: 'Universal EPUB 3 Exporter', slug: 'exporters/epub3' },
            { label: 'Amazon KFX Exporter', slug: 'exporters/kfx' },
            { label: 'Zstd State Caching', slug: 'exporters/zstd' },
            { label: 'CSS & Resource Optimizer', slug: 'exporters/optimizer' },
          ],
        },
        {
          label: 'Model Context Protocol (MCP)',
          items: [
            { label: 'AI Assistant MCP Server', slug: 'mcp/overview' },
            { label: 'Client Setup (Claude/Cursor)', slug: 'mcp/clients' },
          ],
        },
        {
          label: 'SDKs & Language Bindings',
          items: [
            { label: 'Python (PyO3)', slug: 'sdk/python' },
            { label: 'WebAssembly (WASM)', slug: 'sdk/wasm' },
            { label: 'C ABI & UniFFI', slug: 'sdk/uniffi' },
          ],
        },
        {
          label: 'Benchmarks & Parity',
          items: [
            { label: 'Calibre vs ebook-rs Benchmarks', slug: 'benchmarks/conversion' },
            { label: 'Feature Comparison Matrix', slug: 'benchmarks/parity' },
          ],
        },
        {
          label: 'API Reference',
          items: [
            { label: 'Rust API Reference', slug: 'api/reference' },
          ],
        },
      ],
    }),
  ],
});
