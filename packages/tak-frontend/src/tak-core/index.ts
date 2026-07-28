export * from './base';
export * from './game';

export type TakVariant = 'flat' | 'standing' | 'capstone';
export type TakPlayer = 'white' | 'black';
export type TakOpening = 'swap' | 'noSwap' | 'doubleStack';

export interface TakPos {
  x: number;
  y: number;
}

export interface TakReserve {
  pieces: number;
  capstones: number;
}

export interface TakBaseGameSettings {
  boardSize: number;
  halfKomi: number;
  reserve: TakReserve;
  opening: TakOpening;
}

export interface TakGameSettings {
  base: TakBaseGameSettings;
  timeControl: TakTimeControl;
}

export interface TakAsyncTimeControl {
  type: 'async';
  contingentMs: number;
}

export interface TakRealtimeTimeControl {
  type: 'realtime';
  contingentMs: number;
  incrementMs: number;
  extra: {
    onMove: number;
    extraMs: number;
  } | null;
}

export type TakTimeControl = TakAsyncTimeControl | TakRealtimeTimeControl;

export interface TakPieceId {
  type: 'flat' | 'capstone';
  player: TakPlayer;
  kindIndex: number;
  uuid: string;
}

export type TakActionRecord =
  | {
      type: 'place';
      action: TakPlaceAction;
      pieceIds: TakPieceId[];
    }
  | {
      type: 'move';
      action: TakMoveAction;
      pieceIds: TakPieceId[];
      wasSmash: boolean;
    };

interface TakPlaceAction {
  type: 'place';
  pos: TakPos;
  variant: TakVariant;
}
interface TakMoveAction {
  type: 'move';
  pos: TakPos;
  dir: TakDir;
  drops: number[];
}

export type TakAction = TakPlaceAction | TakMoveAction;

export const allDirections = ['up', 'down', 'left', 'right'] as const;
export type TakDir = (typeof allDirections)[number];

export type TakGameResult =
  | { type: 'win'; winner: TakPlayer; reason: 'default' | 'timeout' | 'resignation' }
  | {
      type: 'win';
      winner: TakPlayer;
      reason: 'flats';
      counts?: Record<TakPlayer, number>;
      flats?: TakPos[];
    }
  | { type: 'win'; winner: TakPlayer; reason: 'road'; road?: TakPos[] }
  | { type: 'draw' }
  | { type: 'aborted' };

export type TakGameState = { type: 'ongoing' } | TakGameResult;

export function playerOpponent(player: TakPlayer): TakPlayer {
  return player === 'white' ? 'black' : 'white';
}

export function getDefaultReserve(size: number): TakReserve {
  if (size === 3) return { pieces: 10, capstones: 0 };
  if (size === 4) return { pieces: 15, capstones: 0 };
  if (size === 5) return { pieces: 21, capstones: 1 };
  if (size === 6) return { pieces: 30, capstones: 1 };
  if (size === 7) return { pieces: 40, capstones: 2 };
  if (size === 8) return { pieces: 50, capstones: 2 };
  return { pieces: 21, capstones: 1 };
}

export function isDefaultReserve(size: number, reserve: TakReserve): boolean {
  const defaultReserve = getDefaultReserve(size);
  return reserve.pieces === defaultReserve.pieces && reserve.capstones === defaultReserve.capstones;
}

export function offsetPos(pos: TakPos, dir: TakDir, steps: number): TakPos {
  switch (dir) {
    case 'up':
      return { x: pos.x, y: pos.y + steps };
    case 'down':
      return { x: pos.x, y: pos.y - steps };
    case 'left':
      return { x: pos.x - steps, y: pos.y };
    case 'right':
      return { x: pos.x + steps, y: pos.y };
  }
}

export function isValidPos(size: number, pos: TakPos): boolean {
  return pos.x >= 0 && pos.x < size && pos.y >= 0 && pos.y < size;
}

export function dirFromAdjacent(to: TakPos, from: TakPos): TakDir | null {
  if (to.x === from.x && to.y === from.y + 1) return 'up';
  if (to.x === from.x && to.y === from.y - 1) return 'down';
  if (to.y === from.y && to.x === from.x + 1) return 'right';
  if (to.y === from.y && to.x === from.x - 1) return 'left';
  return null;
}

export function dirFromAligned(to: TakPos, from: TakPos): TakDir | null {
  if (to.x === from.x && to.y > from.y) return 'up';
  if (to.x === from.x && to.y < from.y) return 'down';
  if (to.y === from.y && to.x > from.x) return 'right';
  if (to.y === from.y && to.x < from.x) return 'left';
  return null;
}

export function actionEquals(a: TakAction, b: TakAction | undefined | null): boolean {
  if (!b) return false;
  if (a.type === 'place' && b.type === 'place') {
    return a.variant === b.variant && a.pos.x === b.pos.x && a.pos.y === b.pos.y;
  }
  if (a.type === 'move' && b.type === 'move') {
    return (
      a.pos.x === b.pos.x &&
      a.pos.y === b.pos.y &&
      a.dir === b.dir &&
      a.drops.length === b.drops.length &&
      a.drops.every((drop, index) => drop === b.drops[index])
    );
  }
  return false;
}

export function gameResultEquals(a: TakGameResult, b: TakGameResult | undefined | null): boolean {
  if (!b) return false;
  if (a.type === 'win' && b.type === 'win') {
    return a.winner === b.winner && a.reason === b.reason;
  }
  return a.type === b.type;
}

export function baseGameSettingsEquals(
  a: TakBaseGameSettings,
  b: TakBaseGameSettings | undefined | null,
): boolean {
  if (!b) return false;
  return (
    a.boardSize === b.boardSize &&
    a.halfKomi === b.halfKomi &&
    a.reserve.pieces === b.reserve.pieces &&
    a.reserve.capstones === b.reserve.capstones
  );
}

export function gameSettingsEquals(
  a: TakGameSettings,
  b: TakGameSettings | undefined | null,
): boolean {
  if (!b) return false;
  if (!baseGameSettingsEquals(a.base, b.base)) return false;

  if (a.timeControl.type === 'async' && b.timeControl.type === 'async') {
    return a.timeControl.contingentMs === b.timeControl.contingentMs;
  }
  if (a.timeControl.type === 'realtime' && b.timeControl.type === 'realtime') {
    const extraEqual =
      (a.timeControl.extra === null && b.timeControl.extra === null) ||
      (a.timeControl.extra !== null &&
        b.timeControl.extra !== null &&
        a.timeControl.extra.onMove === b.timeControl.extra.onMove &&
        a.timeControl.extra.extraMs === b.timeControl.extra.extraMs);
    return (
      a.timeControl.contingentMs === b.timeControl.contingentMs &&
      a.timeControl.incrementMs === b.timeControl.incrementMs &&
      extraEqual
    );
  }
  return false;
}
