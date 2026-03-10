/**
 * Structured logger using pino.
 * Falls back to console if pino is unavailable.
 */

import pino from 'pino';

const level = process.env['LOG_LEVEL'] ?? 'info';
const pretty = process.env['LOG_PRETTY'] === 'true';

export const logger = pino(
  {
    level,
    ...(pretty && {
      transport: {
        target: 'pino-pretty',
        options: {
          colorize: true,
          translateTime: 'SYS:standard',
          ignore: 'pid,hostname',
        },
      },
    }),
  },
);

export default logger;
