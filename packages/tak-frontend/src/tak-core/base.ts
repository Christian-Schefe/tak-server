import { immerable } from 'immer';
import {
  playerOpponent,
  type TakAction,
  type TakActionRecord,
  type TakBaseGameSettings,
  type TakGameResult,
  type TakGameState,
  type TakPlayer,
  type TakReserve,
  type TakVariant,
} from '.';
import { TakBoard } from './board';

export class TakBaseGame {
  [immerable] = true;

  settings: TakBaseGameSettings;
  board: TakBoard;
  currentPlayer: TakPlayer = 'white';
  reserves: Record<TakPlayer, TakReserve>;
  boardHashHistory: Record<string, number | undefined> = {};
  actionHistory: TakActionRecord[] = [];
  gameResult: TakGameResult | null = null;

  constructor(settings: TakBaseGameSettings) {
    this.settings = settings;
    this.board = new TakBoard(settings.boardSize);
    this.reserves = {
      white: { ...settings.reserve },
      black: { ...settings.reserve },
    };
  }

  clone(): TakBaseGame {
    const newGame = new TakBaseGame(this.settings);
    newGame.board = this.board.clone();
    newGame.currentPlayer = this.currentPlayer;
    newGame.reserves = {
      white: { ...this.reserves.white },
      black: { ...this.reserves.black },
    };
    newGame.boardHashHistory = { ...this.boardHashHistory };
    newGame.actionHistory = [...this.actionHistory];
    newGame.gameResult = this.gameResult;
    return newGame;
  }

  gameState(): TakGameState {
    return this.gameResult ?? { type: 'ongoing' };
  }

  isOngoing(): boolean {
    return this.gameResult === null;
  }

  canDoAction(action: TakAction): boolean {
    if (this.gameResult !== null) {
      return false;
    }
    const isOpeningMove = this.actionHistory.length < 2;
    const isFirstMove = this.actionHistory.length === 0;
    if (isOpeningMove) {
      switch (action.type) {
        case 'place': {
          if (action.variant !== 'flat') {
            return false;
          }
          const reserve = this.reserves[this.currentPlayer];
          const opponentReserve = this.reserves[playerOpponent(this.currentPlayer)];
          let valid: boolean;
          switch (this.settings.opening) {
            case 'swap':
              valid = opponentReserve.pieces > 0;
              break;
            case 'noSwap':
              valid = reserve.pieces > 0;
              break;
            case 'doubleStack':
              valid = opponentReserve.pieces > (isFirstMove ? 1 : 0);
              break;
          }
          if (!valid) {
            return false;
          }
          return this.board.canDoPlace(action.pos);
        }
        case 'move': {
          return false;
        }
      }
    } else {
      switch (action.type) {
        case 'place': {
          const reserve = this.reserves[this.currentPlayer];
          const amountInReserve =
            action.variant === 'capstone' ? reserve.capstones : reserve.pieces;
          if (amountInReserve <= 0) {
            return false;
          }
          return this.board.canDoPlace(action.pos);
        }
        case 'move': {
          return this.board.canDoMove(action.pos, action.dir, action.drops);
        }
      }
    }
  }

  doAction(action: TakAction) {
    if (!this.canDoAction(action)) {
      throw new Error(`Cannot perform action: ${JSON.stringify(action)}`);
    }
    const movedPlayer = this.currentPlayer;
    const isOpeningMove = this.actionHistory.length < 2;
    const isFirstMove = this.actionHistory.length === 0;
    let actionRecord: TakActionRecord;
    switch (action.type) {
      case 'place': {
        const opponentPlayer = playerOpponent(this.currentPlayer);
        const reserve = this.reserves[this.currentPlayer];
        const opponentReserve = this.reserves[opponentPlayer];
        let placingComposition: TakPlayer[];
        if (isOpeningMove) {
          switch (this.settings.opening) {
            case 'swap':
              placingComposition = [opponentPlayer];
              takeFromReserve(opponentReserve, action.variant, placingComposition.length);
              break;
            case 'noSwap':
              placingComposition = [this.currentPlayer];
              takeFromReserve(reserve, action.variant, placingComposition.length);
              break;
            case 'doubleStack':
              placingComposition = isFirstMove
                ? [opponentPlayer, opponentPlayer]
                : [opponentPlayer];
              takeFromReserve(opponentReserve, action.variant, placingComposition.length);
              break;
          }
        } else {
          placingComposition = [this.currentPlayer];
          takeFromReserve(reserve, action.variant, placingComposition.length);
        }
        const pieceIds = this.board.doPlace(action.pos, action.variant, placingComposition);
        actionRecord = { type: 'place', action, pieceIds };
        break;
      }
      case 'move': {
        const { pieceIds, wasSmash } = this.board.doMove(action.pos, action.dir, action.drops);
        actionRecord = {
          type: 'move',
          action,
          pieceIds,
          wasSmash,
        };
        break;
      }
    }
    this.actionHistory.push(actionRecord);
    this.currentPlayer = playerOpponent(this.currentPlayer);

    const boardHash = this.board.computeHashString();
    this.boardHashHistory[boardHash] = (this.boardHashHistory[boardHash] ?? 0) + 1;

    const gameResult = this.checkGameOver(boardHash, movedPlayer);
    if (gameResult) {
      this.gameResult = gameResult;
    }
    return;
  }

  canUndoAction(): boolean {
    if (this.gameResult !== null) {
      return false;
    }
    return this.actionHistory.length > 0;
  }

  undoAction() {
    if (!this.canUndoAction()) {
      throw new Error('Cannot undo action: no actions to undo or game is over');
    }
    this.trimToPlyIndex(this.actionHistory.length - 1);
  }

  private checkGameOver(boardHash: string, movedPlayer: TakPlayer): TakGameResult | null {
    const whiteReserveEmpty =
      this.reserves.white.pieces === 0 && this.reserves.white.capstones === 0;
    const blackReserveEmpty =
      this.reserves.black.pieces === 0 && this.reserves.black.capstones === 0;
    const repeatCount = this.boardHashHistory[boardHash] ?? 0;
    const maybeRoad = this.board.checkForRoad(movedPlayer);
    const maybeOpponentRoad = this.board.checkForRoad(playerOpponent(movedPlayer));
    if (maybeRoad) {
      return {
        type: 'win',
        winner: movedPlayer,
        reason: 'road',
        road: maybeRoad,
      };
    } else if (maybeOpponentRoad) {
      return {
        type: 'win',
        winner: playerOpponent(movedPlayer),
        reason: 'road',
        road: maybeOpponentRoad,
      };
    } else if (this.board.isFull() || whiteReserveEmpty || blackReserveEmpty) {
      const flatCounts = this.board.countFlats();
      const whiteScore = flatCounts.white * 2;
      const blackScore = flatCounts.black * 2 + this.settings.halfKomi;
      if (whiteScore > blackScore) {
        return { type: 'win', winner: 'white', reason: 'flats', counts: flatCounts };
      } else if (blackScore > whiteScore) {
        return { type: 'win', winner: 'black', reason: 'flats', counts: flatCounts };
      } else {
        return { type: 'draw' };
      }
    } else if (repeatCount >= 3) {
      return { type: 'draw' };
    }
    return null;
  }

  trimToPlyIndex(plyIndex: number) {
    const newGame = new TakBaseGame(this.settings);
    // Make sure to copy the UUIDs from the current board to the new board before replaying actions
    // so that the same UUIDs are used for pieces in the new game state.
    newGame.board.addUuids(this.board.uuids);
    for (let i = 0; i < plyIndex; i++) {
      const action = this.actionHistory[i];
      if (!action) {
        throw new Error('No actions to undo');
      }
      newGame.doAction(action.action);
    }
    this.board = newGame.board;
    // Trim the UUIDs to the number of pieces that have been placed in the new game state
    // to prevent new pieces from being assigned UUIDs that have already been used in the previous game state.
    this.board.trimExcessUuids();

    this.currentPlayer = newGame.currentPlayer;
    this.reserves = newGame.reserves;
    this.boardHashHistory = newGame.boardHashHistory;
    this.actionHistory = newGame.actionHistory;
    // Don't overwrite gameResult, undo is only possible on ongoing games,
    // and trimming is also used on finished games to trim the history,
    // but the gameResult should remain the same.
  }
}

function takeFromReserve(reserve: TakReserve, variant: TakVariant, amount: number): boolean {
  switch (variant) {
    case 'flat':
      if (reserve.pieces < amount) {
        return false;
      }
      reserve.pieces -= amount;
      break;
    case 'standing':
      if (reserve.pieces < amount) {
        return false;
      }
      reserve.pieces -= amount;
      break;
    case 'capstone':
      if (reserve.capstones < amount) {
        return false;
      }
      reserve.capstones -= amount;
      break;
  }
  return true;
}
