# Technology Stack

**Analysis Date:** 2026-02-04

## Languages

**Primary:**
- TypeScript 5.0 - Full codebase including server and client components

**Secondary:**
- JavaScript - Build configuration and server startup scripts
- HTML - Inline dashboard (simple-server.js)

## Runtime

**Environment:**
- Node.js 24.11.1 (ESM modules with `"type": "module"` in package.json)

**Package Manager:**
- pnpm (via pnpm-lock.yaml lockfile)
- Lockfile: Present (`pnpm-lock.yaml`)

## Frameworks

**Core:**
- SvelteKit 2.0 - Full-stack web framework with file-based routing
- Svelte 5.0 - Component framework and templating engine
- Express 4.21.0 - HTTP server for API routes and middleware

**Build/Dev:**
- Vite 6.0 - Build tool and dev server
- @sveltejs/vite-plugin-svelte 4.0 - Svelte compilation plugin
- @sveltejs/kit 2.0 - Framework core
- @sveltejs/adapter-node 5.2.0 - Node.js deployment adapter

**Testing:**
- svelte-check 4.0 - Type checking and component validation

**UI/Terminal:**
- @xterm/xterm 5.5.0 - Terminal emulation library
- @xterm/addon-fit 0.10.0 - Terminal fit addon for responsive sizing

## Key Dependencies

**Authentication & Security:**
- bcryptjs 3.0.3 - Password hashing (imported but currently not used in verification)
- @types/bcryptjs 3.0.0 - TypeScript types

**Networking:**
- ws 8.18.0 - WebSocket server and client
- cookie-parser 1.4.7 - Cookie middleware for Express

**Development:**
- TypeScript - Type system
- svelte-check - Component and type validation

## Configuration

**Environment:**
- `.env` file for runtime configuration (not committed)
- `.env.example` for documenting required variables

**Environment Variables:**
- `ITERM_TOKEN` - Bearer token for iTerm2 client authentication
- `ADMIN_PASSWORD` - Password for web dashboard login
- `SESSION_SECRET` - Session secret (defined in .env.example but currently unused in session generation)

**Build:**
- `tsconfig.json` - TypeScript compiler options
  - Strict mode enabled
  - ES modules via bundler resolution
  - Source maps enabled
- `svelte.config.js` - SvelteKit configuration
  - Uses vitePreprocess for Svelte preprocessing
  - Configured with @sveltejs/adapter-node
  - CSRF protection enabled with origin checking disabled for local dev
- `vite.config.ts` - Vite configuration with SvelteKit plugin

## Platform Requirements

**Development:**
- Node.js 24.11.1
- pnpm package manager
- TypeScript support

**Production:**
- Node.js 24.x LTS (minimum based on current version)
- Deployment via Node.js runtime (via adapter-node)
- Requires environment variables: ITERM_TOKEN, ADMIN_PASSWORD, SESSION_SECRET

## Scripts

**Development:**
```bash
npm run dev          # Start dev server with Vite
npm run check        # Run type checking with svelte-check
npm run check:watch  # Watch mode for type checking
```

**Build & Deployment:**
```bash
npm run build        # Build SvelteKit app for production
npm run preview      # Preview production build locally
npm start            # Start production server (node server.js)
```

---

*Stack analysis: 2026-02-04*
