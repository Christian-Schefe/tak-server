import { current, immerable, isDraft } from 'immer';
import {
  dirFromAdjacent,
  isValidPos,
  offsetPos,
  type TakAction,
  type TakDir,
  type TakPlayer,
  type TakPos,
  type TakVariant,
} from '.';
import { TakBaseGame } from './base';

export interface TakUIPiece {
  player: TakPlayer;
  variant: TakVariant;
  pos: TakPos;
  height: number;
  isFloating: boolean;
  zPriority: number | null;
  deleted: boolean;
  buriedPieceCount: number;
  canBePicked: boolean;
}

export interface TakUITile {
  owner: TakPlayer | null;
  highlighted: boolean;
  selectable: boolean;
  hoverable: boolean;
  lastAction: boolean;
}

export class TakGameUI {
  [immerable] = true;

  actualGame: TakBaseGame;
  pieces: Record<string, TakUIPiece | undefined>;
  priorityPieces: string[];
  tiles: TakUITile[];
  partialAction: PartialAction | null;

  constructor(game: TakBaseGame) {
    this.actualGame = game;
    this.pieces = {};
    this.priorityPieces = [];
    this.partialAction = null;
    this.tiles = [];
    const size = game.board.size;
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        this.tiles.push({
          owner: null,
          highlighted: false,
          selectable: false,
          hoverable: false,
          lastAction: false,
        });
      }
    }
    this.onGameUpdate();
  }

  updateGame(game: TakBaseGame) {
    if (this.actualGame === game) {
      return;
    }
    const isSteppingForwardOne =
      game.actionHistory.length === this.actualGame.actionHistory.length + 1;
    const isSteppingBackOne =
      game.actionHistory.length === this.actualGame.actionHistory.length - 1;

    this.priorityPieces = (
      isSteppingForwardOne
        ? (game.actionHistory[game.actionHistory.length - 1]?.pieceIds ?? [])
        : isSteppingBackOne
          ? (this.actualGame.actionHistory[this.actualGame.actionHistory.length - 1]?.pieceIds ??
            [])
          : []
    ).map((id) => id.uuid);

    this.actualGame = game;
    this.partialAction = null;
    this.onGameUpdate();
  }

  private onGameUpdate() {
    const shownGame = isDraft(this.actualGame)
      ? current(this.actualGame).clone()
      : this.actualGame.clone();

    const partialAction = partialActionToAction(this.partialAction);
    if (partialAction) {
      shownGame.doAction(partialAction.action);
      this.priorityPieces = getLastActionPiecesInOrder(shownGame);
    }

    const floatingData = this.partialAction && {
      pos: this.partialAction.dir
        ? offsetPos(this.partialAction.pos, this.partialAction.dir, this.partialAction.drops.length)
        : this.partialAction.pos,
      floatingCount:
        this.partialAction.take - this.partialAction.drops.reduce((acc, drop) => acc + drop, 0),
    };

    const size = this.actualGame.board.size;

    const clickOptions = [];

    if (this.partialAction && floatingData) {
      const clickOptionDirs: TakDir[] = this.partialAction.dir
        ? [this.partialAction.dir]
        : ['left', 'up', 'down', 'right'];
      const dropPos = floatingData.pos;
      const floatingCount = floatingData.floatingCount;
      clickOptions.push(dropPos);
      for (const dir of clickOptionDirs) {
        const newPos = offsetPos(dropPos, dir, 1);
        if (!isValidPos(size, newPos)) continue;
        const stack = shownGame.board.getStack(newPos);
        if (!stack || stack.variant === 'flat') {
          clickOptions.push(newPos);
        } else if (stack.variant === 'standing' && floatingCount === 1) {
          const thisStack = shownGame.board.getStack(dropPos);
          if (thisStack && thisStack.variant === 'capstone') {
            clickOptions.push(newPos);
          }
        }
      }
    }

    const isOngoing = !this.actualGame.gameResult;

    const presentIds = new Set<string>();

    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        const stack = shownGame.board.getStack({ x, y });
        const pos = { x, y };
        const index = y * size + x;
        const selectable = clickOptions.some((pos) => pos.x === x && pos.y === y);
        let hoverable = this.partialAction === null;
        if (stack) {
          const floatingHeightThreshold =
            floatingData && pos.x === floatingData.pos.x && pos.y === floatingData.pos.y
              ? stack.composition.length - floatingData.floatingCount
              : null;
          const buriedPieceCount = Math.max(0, stack.composition.length - size);

          for (let height = 0; height < stack.composition.length; height++) {
            const piece = stack.composition[height];
            if (!piece) continue;
            const priorityIndex = this.priorityPieces.findIndex((id) => id === piece.uuid);
            const canBePicked = stack.composition.length - height <= size;
            const id = piece.uuid;
            const newPiece: TakUIPiece = {
              buriedPieceCount,
              canBePicked,
              zPriority: priorityIndex >= 0 ? priorityIndex : null,
              player: piece.player,
              variant: height === stack.composition.length - 1 ? stack.variant : 'flat',
              pos,
              height,
              isFloating: floatingHeightThreshold !== null && height >= floatingHeightThreshold,
              deleted: false,
            };
            if (arePiecesDifferent(this.pieces[id], newPiece)) {
              this.pieces[id] = newPiece;
            }
            presentIds.add(id);
          }
          hoverable &&=
            this.actualGame.actionHistory.length >= 2 &&
            stack.composition[stack.composition.length - 1]?.player ===
              this.actualGame.currentPlayer;
        }

        const newTile: TakUITile = {
          owner: stack?.composition[0]?.player ?? null,
          highlighted: false,
          selectable: isOngoing && selectable,
          hoverable: isOngoing && (hoverable || selectable),
          lastAction: false,
        };
        if (areTilesDifferent(this.tiles[index], newTile)) {
          this.tiles[index] = newTile;
        }
      }
    }

    for (const id of Object.keys(this.pieces)) {
      if (this.pieces[id] !== undefined && !presentIds.has(id)) {
        if (this.pieces[id].deleted) {
          // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
          delete this.pieces[id];
        } else {
          this.pieces[id].deleted = true;
        }
      }
    }

    if (this.actualGame.actionHistory.length >= 1) {
      const lastAction = this.actualGame.actionHistory[this.actualGame.actionHistory.length - 1];
      if (lastAction?.action.type === 'place') {
        const lastActionPosIndex = lastAction.action.pos.y * size + lastAction.action.pos.x;
        const lastActionTile = this.tiles[lastActionPosIndex];
        if (lastActionTile) {
          lastActionTile.lastAction = true;
        }
      } else if (lastAction) {
        for (let i = 0; i <= lastAction.action.drops.length; i++) {
          const pos = offsetPos(lastAction.action.pos, lastAction.action.dir, i);
          const posIndex = pos.y * size + pos.x;
          const tile = this.tiles[posIndex];
          if (tile) {
            tile.lastAction = true;
          }
        }
      }
    }
  }

  tryPlaceOrAddToPartialAction(pos: TakPos, variant: TakVariant | null): TakAction | null {
    const action: TakAction = {
      type: 'place',
      pos,
      variant: variant ?? 'flat',
    };
    if (variant && !this.partialAction && this.actualGame.canDoAction(action)) {
      return action;
    } else {
      return this.getPartialAction(pos);
    }
  }

  updatePartialAction(pos: TakPos) {
    const newPartialAction = this.getNewPartialAction(pos);
    this.partialAction = newPartialAction;
    this.onGameUpdate();
  }

  private getPartialAction(pos: TakPos): TakAction | null {
    const newPartialAction = this.getNewPartialAction(pos);
    const partialAction = partialActionToAction(newPartialAction);
    if (partialAction?.complete === true) {
      return partialAction.action;
    }
    return null;
  }

  private getNewPartialAction(pos: TakPos): PartialAction | null {
    let partialAction: PartialAction | null = this.partialAction
      ? { ...this.partialAction, drops: [...this.partialAction.drops] }
      : null;

    if (!this.actualGame.isOngoing() || this.actualGame.actionHistory.length < 2) {
      partialAction = null;
      return partialAction;
    }

    if (!partialAction) {
      const stack = this.actualGame.board.getStack(pos);
      if (!stack) return partialAction;

      if (
        stack.composition[stack.composition.length - 1]?.player !== this.actualGame.currentPlayer
      ) {
        return partialAction;
      }
      partialAction = {
        take: Math.min(stack.composition.length, this.actualGame.board.size),
        drops: [],
        pos,
        dir: null,
      };
      return partialAction;
    }

    const stack = this.actualGame.board.getStack(partialAction.pos);
    if (!stack) {
      partialAction = null;
      return partialAction;
    }

    const dropPos = partialAction.dir
      ? offsetPos(partialAction.pos, partialAction.dir, partialAction.drops.length)
      : partialAction.pos;
    if (dropPos.x === pos.x && dropPos.y === pos.y) {
      if (partialAction.drops.length > 0) {
        const newDrops = (partialAction.drops[partialAction.drops.length - 1] ?? 0) + 1;
        partialAction.drops[partialAction.drops.length - 1] = newDrops;
      } else {
        partialAction.take--;
        if (partialAction.take <= 0) {
          partialAction = null;
          return partialAction;
        }
      }
    } else {
      const dir = dirFromAdjacent(pos, dropPos);
      if (!dir || (partialAction.dir && partialAction.dir !== dir)) {
        partialAction = null;
        return partialAction;
      }
      const otherStack = this.actualGame.board.getStack(pos);
      if (otherStack && otherStack.variant !== 'flat') {
        const floatingCount =
          partialAction.take - partialAction.drops.reduce((acc, drop) => acc + drop, 0);
        if (
          !(
            stack.variant === 'capstone' &&
            otherStack.variant === 'standing' &&
            floatingCount === 1
          )
        ) {
          partialAction = null;
          return partialAction;
        }
      }
      partialAction.dir = dir;
      partialAction.drops.push(1);
    }
    return partialAction;
  }
}

function partialActionToAction(
  partialAction: PartialAction | null,
): { action: TakAction; complete: boolean } | null {
  if (partialAction?.dir) {
    const drops = partialAction.drops;
    const floatingCount = partialAction.take - drops.reduce((acc, drop) => acc + drop, 0);

    return {
      action: {
        pos: partialAction.pos,
        type: 'move',
        dir: partialAction.dir,
        drops: drops.map((x, i) => (i === drops.length - 1 ? x + floatingCount : x)),
      },
      complete: floatingCount === 0,
    };
  }
  return null;
}

function getLastActionPiecesInOrder(game: TakBaseGame): string[] {
  if (game.actionHistory.length === 0) return [];
  const lastAction = game.actionHistory[game.actionHistory.length - 1];
  return lastAction?.pieceIds.map((id) => id.uuid) ?? [];
}

interface PartialAction {
  take: number;
  drops: number[];
  pos: TakPos;
  dir: TakDir | null;
}

function arePiecesDifferent(piece: TakUIPiece | undefined, newData: TakUIPiece): boolean {
  return (
    !piece ||
    piece.player !== newData.player ||
    piece.variant !== newData.variant ||
    piece.pos.x !== newData.pos.x ||
    piece.pos.y !== newData.pos.y ||
    piece.height !== newData.height ||
    piece.isFloating !== newData.isFloating ||
    piece.zPriority !== newData.zPriority ||
    piece.deleted !== newData.deleted ||
    piece.buriedPieceCount !== newData.buriedPieceCount ||
    piece.canBePicked !== newData.canBePicked
  );
}

function areTilesDifferent(tile: TakUITile | undefined, newTile: TakUITile): boolean {
  return (
    !tile ||
    tile.owner !== newTile.owner ||
    tile.highlighted !== newTile.highlighted ||
    tile.selectable !== newTile.selectable ||
    tile.hoverable !== newTile.hoverable ||
    tile.lastAction !== newTile.lastAction
  );
}
