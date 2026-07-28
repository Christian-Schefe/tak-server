import {
  getDefaultReserve,
  type TakAction,
  type TakActionRecord,
  type TakBaseGameSettings,
  type TakDir,
  type TakGameResult,
  type TakOpening,
  type TakPlayer,
  type TakVariant,
} from '.';
import { TakBaseGame } from './base';
import { TakBoard, type TakStack } from './board';

export function actionFromString(str: string): TakAction | null {
  function stringToVariant(variant: string): TakVariant | null {
    switch (variant) {
      case 'S':
        return 'standing';
      case 'C':
        return 'capstone';
      case 'F':
        return 'flat';
      default:
        return null;
    }
  }
  function stringToDir(dir: string): TakDir | null {
    switch (dir) {
      case '>':
        return 'right';
      case '<':
        return 'left';
      case '+':
        return 'up';
      case '-':
        return 'down';
      default:
        return null;
    }
  }

  const moveRegex = /^([1-9]?)([a-z])([1-9])([<>+-])([1-9]*)/;
  const moveMatch = moveRegex.exec(str);
  if (moveMatch) {
    const take =
      moveMatch[1] !== undefined && moveMatch[1].length > 0
        ? moveMatch[1].charCodeAt(0) - '0'.charCodeAt(0)
        : 1;
    const x = (moveMatch[2] ?? '').charCodeAt(0) - 'a'.charCodeAt(0);
    const y = (moveMatch[3] ?? '').charCodeAt(0) - '1'.charCodeAt(0);
    const dir = stringToDir(moveMatch[4] ?? '');
    if (!dir) {
      return null;
    }
    const drops =
      moveMatch[5] !== undefined && moveMatch[5].length > 0
        ? moveMatch[5].split('').map((d) => d.charCodeAt(0) - '0'.charCodeAt(0))
        : [take];
    return { type: 'move', pos: { x, y }, dir, drops };
  }

  const placeRegex = /^([FSC]?)([a-z])([1-9])/;
  const placeMatch = placeRegex.exec(str);
  if (placeMatch) {
    const variant = stringToVariant((placeMatch[1] ?? 'F') || 'F');
    if (!variant) {
      return null;
    }
    const x = (placeMatch[2] ?? '').charCodeAt(0) - 'a'.charCodeAt(0);
    const y = (placeMatch[3] ?? '').charCodeAt(0) - '1'.charCodeAt(0);
    return { type: 'place', variant, pos: { x, y } };
  }

  return null;
}

export function actionToString(move: TakAction): string {
  if (move.type === 'place') {
    const variant = move.variant === 'capstone' ? 'C' : move.variant === 'standing' ? 'S' : '';

    const col = String.fromCharCode(move.pos.x + 'a'.charCodeAt(0));
    const row = move.pos.y + 1;
    return `${variant}${col}${row.toString()}`;
  } else {
    const col = String.fromCharCode(move.pos.x + 'a'.charCodeAt(0));
    const row = move.pos.y + 1;
    const dir =
      move.dir === 'up' ? '+' : move.dir === 'down' ? '-' : move.dir === 'left' ? '<' : '>';
    const takeSum = move.drops.reduce((a, b) => a + b, 0);
    const take = takeSum === 1 ? '' : takeSum.toString();
    const drops = move.drops.length === 1 ? '' : move.drops.join('');
    return `${take}${col}${row.toString()}${dir}${drops}`;
  }
}

function openingToString(opening: TakOpening): string {
  switch (opening) {
    case 'swap':
      return 'swap';
    case 'noSwap':
      return 'no-swap';
    case 'doubleStack':
      return 'double black stack';
  }
}

function openingFromString(str: string): TakOpening | null {
  switch (str) {
    case 'swap':
      return 'swap';
    case 'no-swap':
      return 'noSwap';
    case 'double black stack':
      return 'doubleStack';
    default:
      return null;
  }
}

export function gameToPTN(
  settings: TakBaseGameSettings,
  history: TakActionRecord[],
  gameState: TakGameResult | null,
  usernames?: Record<TakPlayer, string>,
) {
  const attributes = [
    { name: 'Size', value: settings.boardSize.toString() },
    { name: 'Komi', value: (settings.halfKomi / 2).toString() },
    { name: 'Flats', value: settings.reserve.pieces.toString() },
    { name: 'Caps', value: settings.reserve.capstones.toString() },
    { name: 'Opening', value: openingToString(settings.opening) },
  ];

  if (gameState) {
    attributes.push({ name: 'Result', value: gameResultToString(gameState) });
  }

  if (usernames) {
    attributes.push(
      { name: 'Player1', value: usernames.white },
      { name: 'Player2', value: usernames.black },
    );
  }

  const moves = history.map((record) => actionToString(record.action));

  const movePairs = [];
  for (let i = 0; i < moves.length; i += 2) {
    const firstMove = moves[i] ?? '';
    const secondMove = moves[i + 1] ?? '';
    movePairs.push(firstMove + (secondMove.length > 0 ? ` ${secondMove}` : ''));
  }
  const moveStr = movePairs.map((pair, index) => `${(index + 1).toString()}. ${pair}`).join('\n');

  return `${attributes.map((attr) => `[${attr.name} "${attr.value}"]`).join('\n')}\n${moveStr}`;
}

const PTN_ATTRIBUTES_REGEX = /(?:\[(\w*)\s"([^"]*)"\])\s*/g;
const PTN_MOVES_REGEX = /(?!\d*\.)[^\s]+/g;

const PTN_GAME_OVER_REGEX = /^(1\/2-1\/2|0-1|1-0|0-F|F-0|0-R|R-0|0-0)/;
const INTEGER_REGEX = /^\d+$/;

export function PTNToGame(ptn: string): {
  game: TakBaseGame;
  playerInfo: Record<TakPlayer, { username: string; rating?: number }>;
} | null {
  const attributeMatches = Array.from(ptn.matchAll(PTN_ATTRIBUTES_REGEX)).map((match) => {
    const [, name, value] = match;
    return [name ?? '', value ?? ''] as const;
  });
  const attributes: Map<string, string> = new Map<string, string>(attributeMatches);
  const size = attributes.get('Size');
  const flats = attributes.get('Flats');
  const caps = attributes.get('Caps');
  const komi = attributes.get('Komi');
  const player1 = attributes.get('Player1') ?? 'Player 1';
  const player2 = attributes.get('Player2') ?? 'Player 2';
  const rating1Str = attributes.get('Rating1');
  const rating2Str = attributes.get('Rating2');
  const openingStr = attributes.get('Opening');
  const rating1 = rating1Str !== undefined ? parseInt(rating1Str) : undefined;
  const rating2 = rating2Str !== undefined ? parseInt(rating2Str) : undefined;
  const playerInfo = {
    white: {
      username: player1,
      rating: rating1 !== undefined && !isNaN(rating1) ? rating1 : undefined,
    },
    black: {
      username: player2,
      rating: rating2 !== undefined && !isNaN(rating2) ? rating2 : undefined,
    },
  };
  if (size === undefined || !INTEGER_REGEX.test(size)) return null;
  const sizeNum = parseInt(size);
  const reserve = getDefaultReserve(sizeNum);
  if (flats !== undefined && INTEGER_REGEX.test(flats)) reserve.pieces = parseInt(flats);
  if (caps !== undefined && INTEGER_REGEX.test(caps)) reserve.capstones = parseInt(caps);
  let halfKomi = 0;
  if (komi !== undefined && !Number.isNaN(parseFloat(komi)))
    halfKomi = Math.floor(parseFloat(komi) * 2);

  const restString = ptn.replaceAll(PTN_ATTRIBUTES_REGEX, '').trim();
  const moveMatches = Array.from(restString.matchAll(PTN_MOVES_REGEX));
  const moves = moveMatches.map((match) => match[0]);
  const gameOverStr = moves.find((move) => PTN_GAME_OVER_REGEX.test(move));
  const filteredMoves = moves
    .filter((move) => !PTN_GAME_OVER_REGEX.test(move))
    .map((move) => actionFromString(move.trim()));
  const gameState = gameResultFromString(gameOverStr ?? '');
  const opening = openingFromString(openingStr ?? 'swap');
  if (!opening) {
    console.error('Invalid opening string in PTN:', openingStr);
    return null;
  }

  const gameSettings: TakBaseGameSettings = {
    boardSize: parseInt(size),
    halfKomi,
    reserve,
    opening,
  };
  const game = new TakBaseGame(gameSettings);
  for (const move of filteredMoves) {
    if (!move) {
      console.error('Invalid move string in PTN:', move);
      return null;
    }
    try {
      game.doAction(move);
    } catch (error) {
      console.error('Error applying move:', move, error);
      return null;
    }
  }
  if (gameState && !game.gameResult) {
    game.gameResult = gameState;
  }
  return { game, playerInfo };
}

export function gameResultToString(gameResult: TakGameResult): string {
  switch (gameResult.type) {
    case 'win': {
      const letter = gameResult.reason === 'flats' ? 'F' : gameResult.reason === 'road' ? 'R' : '1';
      return gameResult.winner === 'white' ? `${letter}-0` : `0-${letter}`;
    }
    case 'draw':
      return '1/2-1/2';
    case 'aborted':
      return '0-0';
  }
}

export type TakPTNGameResult = '1-0' | '0-1' | '1/2-1/2' | '0-F' | 'F-0' | '0-R' | 'R-0' | '0-0';

export function gameResultFromString<T extends string>(
  gameOverStr: T,
): T extends TakPTNGameResult ? TakGameResult : TakGameResult | null {
  switch (gameOverStr) {
    case '1/2-1/2':
      return { type: 'draw' };
    case '0-1':
      return { type: 'win', winner: 'black', reason: 'default' };
    case '1-0':
      return { type: 'win', winner: 'white', reason: 'default' };
    case '0-F':
      return { type: 'win', winner: 'black', reason: 'flats' };
    case 'F-0':
      return { type: 'win', winner: 'white', reason: 'flats' };
    case '0-R':
      return { type: 'win', winner: 'black', reason: 'road' };
    case 'R-0':
      return { type: 'win', winner: 'white', reason: 'road' };
    case '0-0':
      return { type: 'aborted' };
    default:
      return null as T extends TakPTNGameResult ? TakGameResult : null;
  }
}

export function gameToTPS(game: TakBaseGame): string {
  const boardStr = boardToPositionString(game.board);
  const playerStr = game.actionHistory.length % 2 === 0 ? '1' : '2';
  const moveStr = (Math.floor(game.actionHistory.length / 2) + 1).toString();
  return `${boardStr} ${playerStr} ${moveStr}`;
}

export function boardToPositionString(board: TakBoard): string {
  function variantToString(variant: TakVariant) {
    switch (variant) {
      case 'flat':
        return '';
      case 'standing':
        return 'S';
      case 'capstone':
        return 'C';
    }
  }

  function rowToPositionString(row: (TakStack | undefined)[]) {
    const result: string[] = [];
    let emptyCount = 0;

    for (const stack of row) {
      if (stack === undefined) {
        emptyCount++;
      } else {
        if (emptyCount > 0) {
          result.push(`x${emptyCount === 1 ? '' : emptyCount.toString()}`);
          emptyCount = 0;
        }
        result.push(
          `${stack.composition
            .map((piece) => (piece.player === 'white' ? '1' : '2'))
            .join('')}${variantToString(stack.variant)}`,
        );
      }
    }

    if (emptyCount > 0) {
      result.push(`x${emptyCount === 1 ? '' : emptyCount.toString()}`);
    }

    return result.join(',');
  }
  const rows = [];
  for (let y = board.size - 1; y >= 0; y--) {
    const row = board.stacks.slice(y * board.size, (y + 1) * board.size);
    rows.push(rowToPositionString(row));
  }
  return rows.join('/');
}
