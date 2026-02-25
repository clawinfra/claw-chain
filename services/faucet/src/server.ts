/**
 * Express application factory.
 * Wires up middleware, routes, static serving, sessions, and passport.
 */

import express, { Express, Request, Response, NextFunction } from 'express';
import session from 'express-session';
import passport from 'passport';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import type { ApiPromise } from '@polkadot/api';
import type { Database } from 'better-sqlite3';
import type { Config } from './config.js';
import { ipRateLimitMiddleware } from './middleware/rateLimit.js';
import { faucetRouter } from './routes/faucet.js';
import { statusRouter } from './routes/status.js';
import { authRouter } from './routes/auth.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Dynamically import connect-sqlite3 (CommonJS module)
async function getSqliteSessionStore(
  sessionFn: typeof session,
): Promise<session.Store> {
  // @ts-expect-error: connect-sqlite3 has loose typings
  const connectSqlite3 = (await import('connect-sqlite3')).default;
  const SqliteStore = connectSqlite3(sessionFn);
  return new SqliteStore({
    db: 'sessions.db',
    dir: './',
  }) as session.Store;
}

export async function createApp(
  config: Config,
  db: Database,
  api: ApiPromise,
): Promise<Express> {
  const app = express();

  // ── Trust proxy (for X-Forwarded-For) ─────────────────────────────────────
  app.set('trust proxy', 1);

  // ── Body parsing ───────────────────────────────────────────────────────────
  app.use(express.json());
  app.use(express.urlencoded({ extended: false }));

  // ── Sessions ───────────────────────────────────────────────────────────────
  const sessionStore = await getSqliteSessionStore(session);
  app.use(
    session({
      store: sessionStore,
      secret: config.sessionSecret,
      resave: false,
      saveUninitialized: false,
      cookie: {
        secure: process.env.NODE_ENV === 'production',
        httpOnly: true,
        maxAge: 7 * 24 * 60 * 60 * 1000, // 7 days
        sameSite: 'lax',
      },
    }),
  );

  // ── Passport (OAuth) ───────────────────────────────────────────────────────
  app.use(passport.initialize());
  app.use(passport.session());

  // ── Auth routes (no rate limit on auth) ───────────────────────────────────
  app.use('/auth', authRouter({ config }));

  // ── IP rate limit (applied to faucet only) ────────────────────────────────
  const rateLimitMiddleware = ipRateLimitMiddleware({ db, config });

  // ── Faucet route ──────────────────────────────────────────────────────────
  app.use('/faucet', rateLimitMiddleware, faucetRouter({ db, api, config }));

  // ── Status route ──────────────────────────────────────────────────────────
  app.use('/status', statusRouter({ db, api, config }));

  // ── Static frontend ────────────────────────────────────────────────────────
  // Serve from dist/public in production, src/public in dev
  const publicDir = join(__dirname, '..', 'public');
  app.use(express.static(publicDir));

  // Catch-all: serve index.html for unknown routes (SPA fallback)
  app.get('*', (_req: Request, res: Response) => {
    res.sendFile(join(publicDir, 'index.html'));
  });

  // ── Global error handler ──────────────────────────────────────────────────
  app.use((err: Error, _req: Request, res: Response, _next: NextFunction) => {
    console.error(`[${new Date().toISOString()}] Unhandled error:`, err);
    res.status(500).json({ error: 'Internal server error' });
  });

  return app;
}
