import { gameBaseSettingsSchema } from '@/api/game';
import type { TakBaseGame, TakBaseGameSettings } from '@/tak-core';
import { gameToTPS } from '@/tak-core/ptn';
import { z } from 'zod';
import EngineWorker from '@/engine.worker?worker';

const workerResponse = z.union([
  z.object({
    type: z.literal('evaluation'),
    variations: z.array(
      z.object({
        moves: z.array(z.string()),
        evaluation: z.number(),
      }),
    ),
    key: z.string(),
  }),
  z.object({
    type: z.literal('checkSettings'),
    supported: z.boolean(),
  }),
  z.object({
    type: z.literal('initialized'),
  }),
]);

export const workerInput = z.union([
  z.object({
    type: z.literal('evaluate'),
    key: z.string(),
    game: z.object({
      settings: gameBaseSettingsSchema,
      tps: z.string(),
    }),
  }),
  z.object({
    type: z.literal('stop'),
  }),
  z.object({
    type: z.literal('initialize'),
  }),
  z.object({
    type: z.literal('checkSettings'),
    settings: gameBaseSettingsSchema,
  }),
]);

export type FullWorkerResponse = z.infer<typeof workerResponse>;

export type WorkerResponse = Exclude<FullWorkerResponse, { type: 'initialized' }>;

const workers = new Map<string, Worker>();

async function getWorker(id: string): Promise<Worker> {
  const worker = workers.get(id);
  if (worker) {
    return worker;
  }

  const newWorker = new EngineWorker();
  workers.set(id, newWorker);

  const initPromise = new Promise((resolve, reject) => {
    newWorker.onmessage = ({ data }) => {
      const parsedResponse = workerResponse.safeParse(data);

      if (parsedResponse.success && parsedResponse.data.type === 'initialized') {
        console.log('Worker initialized:', id);
        resolve(newWorker);
      }
      if (parsedResponse.success) {
        reject(
          new Error(
            `Invalid response from worker: expectend 'initialized', got: ${parsedResponse.data.type}`,
          ),
        );
      } else {
        reject(new Error(`Invalid response from worker: error: ${parsedResponse.error}`));
      }
    };
  });
  newWorker.postMessage({ type: 'initialize' });
  await initPromise;
  return newWorker;
}

export async function initializeEngine(id: string, callback: (message: WorkerResponse) => void) {
  const worker = await getWorker(id);
  worker.onmessage = ({ data }) => {
    const parsed = workerResponse.safeParse(data);
    if (!parsed.success) {
      console.error('Invalid response from worker:', data, parsed.error);
      throw new Error('Invalid response from worker');
    }

    if (parsed.data.type !== 'initialized') {
      callback(parsed.data);
    }
  };
}

export async function checkEngineSettings(id: string, settings: TakBaseGameSettings) {
  const worker = await getWorker(id);
  const input: z.infer<typeof workerInput> = {
    type: 'checkSettings',
    settings: {
      boardSize: settings.boardSize,
      halfKomi: settings.halfKomi,
      pieces: settings.reserve.pieces,
      capstones: settings.reserve.capstones,
      opening: settings.opening,
    },
  };
  worker.postMessage(input);
}

export async function stopEngine(id: string) {
  const worker = await getWorker(id);
  const input: z.infer<typeof workerInput> = {
    type: 'stop',
  };
  worker.postMessage(input);
}

export async function evaluatePosition(id: string, key: string, game: TakBaseGame) {
  const worker = await getWorker(id);
  const input: z.infer<typeof workerInput> = {
    type: 'evaluate',
    key,
    game: {
      settings: {
        boardSize: game.settings.boardSize,
        halfKomi: game.settings.halfKomi,
        pieces: game.settings.reserve.pieces,
        capstones: game.settings.reserve.capstones,
        opening: game.settings.opening,
      },
      tps: gameToTPS(game),
    },
  };
  worker.postMessage(input);
}
