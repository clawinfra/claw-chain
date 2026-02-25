/**
 * GitHub OAuth routes via passport-github2.
 *
 * GET /auth/github          → redirect to GitHub
 * GET /auth/github/callback → handle callback, set session, redirect to /
 * GET /auth/logout          → destroy session, redirect to /
 * GET /auth/me              → { authenticated, username? }
 */

import { Router, Request, Response, NextFunction } from 'express';
import passport from 'passport';
import { Strategy as GitHubStrategy, Profile } from 'passport-github2';
import type { VerifyCallback } from 'passport-oauth2';
import type { Config } from '../config.js';

interface Deps {
  config: Config;
}

// Module augmentation for express-session
declare module 'express-session' {
  interface SessionData {
    githubUser?: { username: string; id: number };
    pendingBoost?: boolean;
  }
}

export function authRouter(deps: Deps): Router {
  const router = Router();
  const { config } = deps;

  // Only configure GitHub strategy if credentials are present
  if (config.githubClientId && config.githubClientSecret) {
    passport.use(
      new GitHubStrategy(
        {
          clientID: config.githubClientId,
          clientSecret: config.githubClientSecret,
          callbackURL: 'https://faucet.clawchain.win/auth/github/callback',
        },
        (
          _accessToken: string,
          _refreshToken: string,
          profile: Profile,
          done: VerifyCallback,
        ) => {
          done(null, {
            username: profile.username ?? profile.displayName ?? 'unknown',
            id: parseInt(profile.id, 10),
          });
        },
      ),
    );
  }

  // Serialize/deserialize for session
  passport.serializeUser((user, done) => {
    done(null, user);
  });
  passport.deserializeUser((obj: Express.User, done) => {
    done(null, obj);
  });

  // ── GET /auth/github ───────────────────────────────────────────────────────
  router.get(
    '/github',
    (req: Request, res: Response, next: NextFunction) => {
      if (!config.githubClientId) {
        res.redirect('/?error=github_oauth_not_configured');
        return;
      }
      // Mark that user wants a boost after auth
      req.session.pendingBoost = true;
      next();
    },
    passport.authenticate('github', { scope: ['read:user'] }),
  );

  // ── GET /auth/github/callback ──────────────────────────────────────────────
  router.get(
    '/github/callback',
    passport.authenticate('github', { session: false, failureRedirect: '/?error=auth_failed' }),
    (req: Request, res: Response) => {
      const user = req.user as { username: string; id: number } | undefined;
      if (user) {
        req.session.githubUser = user;
      }
      const pending = req.session.pendingBoost;
      req.session.pendingBoost = undefined;
      res.redirect(pending ? '/?boost=1' : '/');
    },
  );

  // ── GET /auth/logout ───────────────────────────────────────────────────────
  router.get('/logout', (req: Request, res: Response) => {
    req.session.destroy(() => {
      res.redirect('/');
    });
  });

  // ── GET /auth/me ───────────────────────────────────────────────────────────
  router.get('/me', (req: Request, res: Response) => {
    const githubUser = req.session.githubUser;
    if (githubUser) {
      res.json({ authenticated: true, username: githubUser.username });
    } else {
      res.json({ authenticated: false });
    }
  });

  return router;
}
